use super::download_utils::{get_platform_id, load_config, verify_sha256};
use crate::ipc_state::update_download_status;
use crate::paths::{get_app_data_dir, get_bin_dir, get_llama_binary_path};
use crate::types::DownloadProgress;
use futures_util::StreamExt;
use std::fs;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

/// Create HTTP client for llama.cpp downloads
fn create_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// Get the path to the version file
fn get_version_file_path() -> Result<std::path::PathBuf, String> {
    let bin_dir = get_bin_dir().map_err(|e| e.to_string())?;
    Ok(bin_dir.join("llama-version.txt"))
}

/// Read the currently installed llama.cpp version
fn read_installed_version() -> Result<String, String> {
    let version_file = get_version_file_path()?;
    if !version_file.exists() {
        return Err("Version file not found".to_string());
    }
    fs::read_to_string(version_file)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Failed to read version file: {}", e))
}

/// Write the installed llama.cpp version
fn write_installed_version(version: &str) -> Result<(), String> {
    let version_file = get_version_file_path()?;
    fs::write(version_file, version).map_err(|e| format!("Failed to write version file: {}", e))
}

/// Check if llama.cpp needs to be updated
fn needs_update(current_version: &str) -> Result<bool, String> {
    match read_installed_version() {
        Ok(installed_version) => Ok(installed_version != current_version),
        Err(_) => Ok(true), // If we can't read version, assume update is needed
    }
}

/// Remove old llama.cpp files
fn cleanup_old_llama_files(bin_dir: &std::path::Path) -> Result<(), String> {
    // Try both with and without .exe extension for cross-platform compatibility
    #[cfg(target_os = "windows")]
    let binary_names = vec!["llama-server.exe", "llama-server"];
    
    #[cfg(not(target_os = "windows"))]
    let binary_names = vec!["llama-server"];

    for name in binary_names {
        let binary_path = bin_dir.join(name);
        if binary_path.exists() {
            if let Err(e) = fs::remove_file(&binary_path) {
                log::warn!("Failed to remove old binary {}: {}", name, e);
            }
        }
    }

    // Remove old .dylib and .metal files
    if let Ok(entries) = fs::read_dir(bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "dylib" || ext == "metal" {
                    if let Err(e) = fs::remove_file(&path) {
                        log::warn!("Failed to remove {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Extract llama-server and related files from archive
fn extract_llama_archive(
    archive: &mut zip::ZipArchive<std::fs::File>,
    bin_dir: &std::path::Path,
) -> Result<(), String> {
    let mut found_server = false;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read file from archive: {}", e))?;

        let file_name = file.name().to_string();

        // Skip directories
        if file_name.ends_with("/") {
            continue;
        }

        // Extract llama-server (with or without .exe), .dylib files, .dll files, and .metal files
        let should_extract = file_name.ends_with("llama-server")
            || file_name.ends_with("llama-server.exe")
            || file_name.ends_with(".dylib")
            || file_name.ends_with(".dll")
            || file_name.ends_with(".metal");

        if should_extract {
            // Get just the filename without the path
            let filename = std::path::Path::new(&file_name)
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("Invalid filename: {}", file_name))?;

            let output_path = bin_dir.join(filename);

            log::info!("Extracting: {} -> {:?}", file_name, output_path);

            let mut outfile = std::fs::File::create(&output_path)
                .map_err(|e| format!("Failed to create output file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;

            // Check if this is the server binary (with or without .exe)
            if filename == "llama-server" || filename == "llama-server.exe" {
                found_server = true;
            }
        }
    }

    if !found_server {
        return Err("llama-server binary not found in archive".to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn check_llama_version() -> Result<bool, String> {
    let config = load_config()?;
    let version = &config.llama_cpp.version;

    needs_update(version)
}

#[tauri::command]
pub async fn download_llama_cpp(app: AppHandle) -> Result<String, String> {
    let bin_dir = get_bin_dir().map_err(|e| e.to_string())?;
    let app_dir = get_app_data_dir().map_err(|e| e.to_string())?;

    // Load llama.cpp configuration
    let config = load_config()?;
    let platform_id = get_platform_id()?;

    // Get the platform-specific configuration
    let platform_config = config
        .llama_cpp
        .platforms
        .get(&platform_id)
        .ok_or_else(|| format!("Platform '{}' not supported in configuration", platform_id))?;

    let version = &config.llama_cpp.version;
    let url = &platform_config.url;

    let binary_path = get_llama_binary_path().map_err(|e| e.to_string())?;

    // Check if llama.cpp is already installed with the correct version
    if binary_path.exists() && !needs_update(version)? {
        return Ok(format!("llama.cpp version {} is already installed", version));
    }

    // If we need to update, remove old files
    if binary_path.exists() {
        let old_version = read_installed_version().unwrap_or_else(|_| "unknown".to_string());
        log::info!(
            "Updating llama.cpp from version {} to {}...",
            old_version, version
        );
        cleanup_old_llama_files(&bin_dir)?;
    }

    let zip_path = app_dir.join("llama-server.zip");

    log::info!("Downloading llama.cpp from: {}", url);

    // Create HTTP client with proper headers
    let client = create_http_client()?;

    // Download zip file with streaming
    let response = client
        .get(url)
        .header("Accept", "*/*")
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    // Check HTTP status
    let status = response.status();
    log::info!("HTTP response status: {}", status);
    
    if !status.is_success() {
        return Err(format!("HTTP error: {} - {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown")));
    }

    let total_size = response.content_length();
    
    if let Some(size) = total_size {
        log::info!("llama.cpp archive size: {:.2} MB", size as f64 / 1_048_576.0);
    } else {
        log::warn!("llama.cpp archive size: unknown (no Content-Length header)");
    }

    // Log some response headers for debugging
    log::info!("Content-Type: {:?}", response.headers().get("content-type"));
    log::info!("Content-Encoding: {:?}", response.headers().get("content-encoding"));

    // Update IPC state - download started
    let _ = update_download_status(true, Some(0.0));

    // Emit initial progress
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            downloaded: 0,
            total: total_size,
            percentage: Some(0.0),
            message: "Starting llama.cpp download...".to_string(),
        },
    );

    let mut file = tokio::fs::File::create(&zip_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Failed to read chunk: {}", e))?;

        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;

        // Emit progress every chunk
        let percentage = total_size.map(|total| (downloaded as f64 / total as f64) * 100.0);
        
        // Update IPC state with progress
        let _ = update_download_status(true, percentage);

        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                downloaded,
                total: total_size,
                percentage,
                message: format!(
                    "Downloading llama.cpp: {:.2} MB",
                    downloaded as f64 / 1_048_576.0
                ),
            },
        );
    }

    // Flush and sync file to ensure all data is written to disk
    file.flush()
        .await
        .map_err(|e| format!("Failed to flush file: {}", e))?;
    
    file.sync_all()
        .await
        .map_err(|e| format!("Failed to sync file: {}", e))?;
    
    // Explicitly close file before verification to ensure all data is persisted
    drop(file);
    
    log::info!("File downloaded successfully: {} bytes", downloaded);

    // Verify SHA-256 checksum
    let expected_hash = &platform_config.sha256;
    
    if !expected_hash.is_empty() {
        if let Err(e) = verify_sha256(&zip_path, expected_hash) {
            // Remove corrupted file
            fs::remove_file(&zip_path).ok();
            // Clear IPC download status on error
            let _ = update_download_status(false, None);
            return Err(format!("Checksum verification failed: {}", e));
        }
    }

    // Emit extraction progress
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            downloaded,
            total: total_size,
            percentage: Some(100.0),
            message: "Extracting llama.cpp binary...".to_string(),
        },
    );

    // Unzip and extract llama-server binary and all required libraries
    let file = match std::fs::File::open(&zip_path) {
        Ok(f) => f,
        Err(e) => {
            let _ = update_download_status(false, None);
            return Err(format!("Failed to open zip file: {}", e));
        }
    };

    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            let _ = update_download_status(false, None);
            return Err(format!("Failed to read zip archive: {}", e));
        }
    };

    if let Err(e) = extract_llama_archive(&mut archive, &bin_dir) {
        let _ = update_download_status(false, None);
        return Err(e);
    }

    // Make executable (Unix-like systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&binary_path)
            .map_err(|e| format!("Failed to get metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&binary_path, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Remove zip file
    fs::remove_file(&zip_path).ok();

    // Write version file to track installed version
    write_installed_version(version)?;

    // Clear IPC download status on success
    let _ = update_download_status(false, None);

    Ok(format!(
        "Downloaded llama.cpp version {} to: {:?}",
        version, binary_path
    ))
}


use crate::paths::{get_app_data_dir, get_bin_dir, get_model_dir};
use crate::types::{DownloadProgress, VersionsConfig};
use futures_util::StreamExt;
use std::fs;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

/// Get current platform identifier for llama.cpp downloads
fn get_platform_id() -> Result<String, String> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok("macos-arm64".to_string());
    
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Ok("macos-x64".to_string());
    
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok("linux-x64".to_string());
    
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok("windows-x64".to_string());
    
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    return Err("Unsupported platform".to_string());
}

/// Load llama.cpp configuration from versions.json
fn load_llama_config() -> Result<VersionsConfig, String> {
    let config_str = include_str!("../versions.json");
    serde_json::from_str(config_str)
        .map_err(|e| format!("Failed to parse versions.json: {}", e))
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
    fs::write(version_file, version)
        .map_err(|e| format!("Failed to write version file: {}", e))
}

/// Check if llama.cpp needs to be updated
fn needs_update(current_version: &str) -> Result<bool, String> {
    match read_installed_version() {
        Ok(installed_version) => Ok(installed_version != current_version),
        Err(_) => Ok(true), // If we can't read version, assume update is needed
    }
}

#[tauri::command]
pub async fn check_llama_version() -> Result<bool, String> {
    let config = load_llama_config()?;
    let version = &config.llama_cpp.version;
    
    needs_update(version)
}

#[tauri::command]
pub async fn download_llama_cpp(app: AppHandle) -> Result<String, String> {
    let bin_dir = get_bin_dir().map_err(|e| e.to_string())?;
    let app_dir = get_app_data_dir().map_err(|e| e.to_string())?;

    // Load llama.cpp configuration
    let config = load_llama_config()?;
    let platform_id = get_platform_id()?;
    
    // Get the platform-specific filename from config
    let filename = config
        .llama_cpp
        .platforms
        .get(&platform_id)
        .ok_or_else(|| format!("Platform '{}' not supported in configuration", platform_id))?;
    
    let version = &config.llama_cpp.version;
    
    // Build GitHub release URL dynamically
    let url = format!(
        "https://github.com/ggml-org/llama.cpp/releases/download/{}/{}",
        version, filename
    );

    let binary_path = bin_dir.join("llama-server");

    // Check if llama.cpp is already installed with the correct version
    if binary_path.exists() && !needs_update(version)? {
        return Ok(format!("llama.cpp version {} is already installed", version));
    }

    // If we need to update, remove old files
    if binary_path.exists() {
        let old_version = read_installed_version().unwrap_or_else(|_| "unknown".to_string());
        println!("Updating llama.cpp from version {} to {}...", old_version, version);
        
        // Remove old binary and related files
        if let Err(e) = fs::remove_file(&binary_path) {
            println!("Warning: Failed to remove old binary: {}", e);
        }
        
        // Remove old .dylib and .metal files
        if let Ok(entries) = fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "dylib" || ext == "metal" {
                        if let Err(e) = fs::remove_file(&path) {
                            println!("Warning: Failed to remove {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    let zip_path = app_dir.join("llama-server.zip");

    // Download zip file with streaming
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    let total_size = response.content_length();

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

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush file: {}", e))?;

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
    let file =
        std::fs::File::open(&zip_path).map_err(|e| format!("Failed to open zip file: {}", e))?;

    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    // Extract llama-server binary and all .dylib files (and .metal for Metal support)
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

        // Extract llama-server, .dylib files, and .metal files
        let should_extract = file_name.ends_with("llama-server")
            || file_name.ends_with(".dylib")
            || file_name.ends_with(".metal");

        if should_extract {
            // Get just the filename without the path
            let filename = std::path::Path::new(&file_name)
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("Invalid filename: {}", file_name))?;

            let output_path = bin_dir.join(filename);

            println!("Extracting: {} -> {:?}", file_name, output_path);

            let mut outfile = std::fs::File::create(&output_path)
                .map_err(|e| format!("Failed to create output file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;

            if filename == "llama-server" {
                found_server = true;
            }
        }
    }

    if !found_server {
        return Err("llama-server binary not found in archive".to_string());
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

    Ok(format!("Downloaded llama.cpp version {} to: {:?}", version, binary_path))
}

#[tauri::command]
pub async fn download_model(model_url: String, app: AppHandle) -> Result<String, String> {
    let model_dir = get_model_dir().map_err(|e| e.to_string())?;
    let zip_path = model_dir.join("model.zip");

    println!("Starting model download from: {}", model_url);
    println!("Download destination: {:?}", zip_path);

    // Create client with headers to ensure proper Content-Length
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Download zip file with streaming
    let response = client
        .get(&model_url)
        .header("Accept", "*/*")
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(|e| format!("Failed to download model: {}", e))?;

    let total_size = response.content_length();

    if let Some(size) = total_size {
        println!("Model size: {:.2} MB", size as f64 / 1_048_576.0);
    } else {
        println!("Model size: unknown");
    }

    // Emit initial progress
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            downloaded: 0,
            total: total_size,
            percentage: Some(0.0),
            message: "Starting model download...".to_string(),
        },
    );

    let mut file = tokio::fs::File::create(&zip_path)
        .await
        .map_err(|e| format!("Failed to create zip file: {}", e))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit_mb = 0u64;
    let mut last_log_mb = 0u64;

    println!("Starting download stream...");

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Failed to read chunk: {}", e))?;

        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;

        // Log progress every 50 MB to console
        let current_log_mb = downloaded / (50 * 1024 * 1024);
        if current_log_mb > last_log_mb {
            last_log_mb = current_log_mb;
            let percentage = total_size.map(|total| (downloaded as f64 / total as f64) * 100.0);
            if let Some(pct) = percentage {
                println!(
                    "Downloaded: {:.2} MB ({:.1}%)",
                    downloaded as f64 / 1_048_576.0,
                    pct
                );
            } else {
                println!("Downloaded: {:.2} MB", downloaded as f64 / 1_048_576.0);
            }
        }

        // Emit progress every 10 MB to reduce event spam
        let current_mb = downloaded / (10 * 1024 * 1024);
        if current_mb > last_emit_mb || total_size.map_or(false, |total| downloaded >= total) {
            last_emit_mb = current_mb;
            let percentage = total_size.map(|total| (downloaded as f64 / total as f64) * 100.0);
            let message = if let Some(total) = total_size {
                format!(
                    "Downloading model: {:.2} MB / {:.2} MB ({:.1}%)",
                    downloaded as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0,
                    percentage.unwrap_or(0.0)
                )
            } else {
                format!(
                    "Downloading model: {:.2} MB",
                    downloaded as f64 / 1_048_576.0
                )
            };

            let _ = app.emit(
                "download-progress",
                DownloadProgress {
                    downloaded,
                    total: total_size,
                    percentage,
                    message,
                },
            );
        }
    }

    println!(
        "Download completed! Total: {:.2} MB",
        downloaded as f64 / 1_048_576.0
    );

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush file: {}", e))?;

    // Emit extraction progress
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            downloaded,
            total: total_size,
            percentage: Some(100.0),
            message: "Extracting model files...".to_string(),
        },
    );

    println!("Starting extraction...");

    // Unzip
    let file =
        std::fs::File::open(&zip_path).map_err(|e| format!("Failed to open zip file: {}", e))?;

    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    let archive_len = archive.len();
    println!("Archive contains {} files", archive_len);

    for i in 0..archive_len {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read file from archive: {}", e))?;

        let outpath = match file.enclosed_name() {
            Some(path) => model_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            println!("Creating directory: {}", file.name());
            fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            println!(
                "Extracting file {}/{}: {} ({:.2} MB)",
                i + 1,
                archive_len,
                file.name(),
                file.size() as f64 / 1_048_576.0
            );
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create output file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
        }
    }

    println!("Extraction completed successfully!");

    // Remove zip file
    println!("Removing temporary zip file...");
    fs::remove_file(&zip_path).ok();

    println!("Model ready at: {:?}", model_dir);
    Ok(format!(
        "Model downloaded and extracted to: {:?}",
        model_dir
    ))
}


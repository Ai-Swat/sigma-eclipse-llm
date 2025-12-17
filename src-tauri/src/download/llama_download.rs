use super::download_utils::{get_platform_id, load_config, verify_sha256};
use crate::ipc_state::update_download_status;
use crate::paths::{get_app_data_dir, get_bin_dir, get_llama_binary_path};
use crate::types::DownloadProgress;
use futures_util::StreamExt;
use std::fs;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

/// Maximum number of retry attempts for chunk read errors
const MAX_CHUNK_RETRIES: u32 = 10;
/// Base delay for exponential backoff (in milliseconds)
const BASE_RETRY_DELAY_MS: u64 = 1000;
/// Maximum delay between retries (in milliseconds)
const MAX_RETRY_DELAY_MS: u64 = 30000;

/// Create HTTP client for llama.cpp downloads
fn create_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(300))
        .connect_timeout(std::time::Duration::from_secs(30))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// Check if server supports Range requests
async fn check_range_support(client: &reqwest::Client, url: &str) -> bool {
    match client.head(url).send().await {
        Ok(response) => {
            let accepts_ranges = response
                .headers()
                .get("accept-ranges")
                .map(|v| v.to_str().unwrap_or("") != "none")
                .unwrap_or(false);
            log::info!("Server range support: {}", accepts_ranges);
            accepts_ranges
        }
        Err(e) => {
            log::warn!("Failed to check range support: {}", e);
            false
        }
    }
}

/// Calculate exponential backoff delay
fn calculate_backoff_delay(attempt: u32) -> std::time::Duration {
    let delay_ms = BASE_RETRY_DELAY_MS * 2u64.pow(attempt.min(10));
    std::time::Duration::from_millis(delay_ms.min(MAX_RETRY_DELAY_MS))
}

/// Start or resume a download request from a given byte offset
async fn start_download_request(
    client: &reqwest::Client,
    url: &str,
    start_byte: u64,
) -> Result<(reqwest::Response, Option<u64>), String> {
    let mut request = client
        .get(url)
        .header("Accept", "*/*")
        .header("Accept-Encoding", "identity");

    if start_byte > 0 {
        log::info!("Resuming download from byte {}", start_byte);
        request = request.header("Range", format!("bytes={}-", start_byte));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    let status = response.status();
    log::info!("HTTP response status: {}", status);

    // 200 OK for new download, 206 Partial Content for resume
    if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(format!(
            "HTTP error: {} - {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown")
        ));
    }

    let total_size = if start_byte > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT {
        // For resumed downloads, parse Content-Range header to get total size
        response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split('/').last())
            .and_then(|s| s.parse::<u64>().ok())
    } else {
        response.content_length()
    };

    Ok((response, total_size))
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

    // Check if server supports range requests for resume capability
    let supports_resume = check_range_support(&client, url).await;

    // Check if partial download exists
    let mut downloaded: u64 = if supports_resume && zip_path.exists() {
        let existing_size = tokio::fs::metadata(&zip_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        if existing_size > 0 {
            log::info!(
                "Found partial download: {:.2} MB, will attempt to resume",
                existing_size as f64 / 1_048_576.0
            );
        }
        existing_size
    } else {
        0
    };

    let (response, total_size) = start_download_request(&client, url, downloaded).await?;

    if let Some(size) = total_size {
        log::info!("llama.cpp archive size: {:.2} MB", size as f64 / 1_048_576.0);
    } else {
        log::warn!("llama.cpp archive size: unknown (no Content-Length header)");
    }

    // Log some response headers for debugging
    log::info!(
        "Content-Type: {:?}",
        response.headers().get("content-type")
    );
    log::info!(
        "Content-Encoding: {:?}",
        response.headers().get("content-encoding")
    );

    // Update IPC state - download started
    let initial_percentage = total_size.map(|total| (downloaded as f64 / total as f64) * 100.0);
    let _ = update_download_status(true, initial_percentage.or(Some(0.0)));

    // Emit initial progress
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            downloaded,
            total: total_size,
            percentage: initial_percentage.or(Some(0.0)),
            message: "Starting llama.cpp download...".to_string(),
        },
    );

    // Open file for writing (append if resuming)
    let mut file = if downloaded > 0 {
        let mut f = tokio::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&zip_path)
            .await
            .map_err(|e| format!("Failed to open zip file for resume: {}", e))?;
        // Seek to end to ensure we're appending
        f.seek(std::io::SeekFrom::End(0))
            .await
            .map_err(|e| format!("Failed to seek to end of file: {}", e))?;
        f
    } else {
        tokio::fs::File::create(&zip_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?
    };

    let mut stream = response.bytes_stream();
    let mut last_emit_mb = downloaded / (10 * 1024 * 1024);
    let mut last_log_mb = downloaded / (50 * 1024 * 1024);
    let mut consecutive_errors = 0u32;

    log::info!("Starting download stream...");

    loop {
        match stream.next().await {
            Some(Ok(chunk)) => {
                // Reset error counter on successful chunk
                consecutive_errors = 0;

                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("Failed to write chunk: {}", e))?;

                downloaded += chunk.len() as u64;

                // Log progress every 50 MB to console
                let current_log_mb = downloaded / (50 * 1024 * 1024);
                if current_log_mb > last_log_mb {
                    last_log_mb = current_log_mb;
                    let percentage =
                        total_size.map(|total| (downloaded as f64 / total as f64) * 100.0);
                    if let Some(pct) = percentage {
                        log::info!(
                            "Downloaded: {:.2} MB ({:.1}%)",
                            downloaded as f64 / 1_048_576.0,
                            pct
                        );
                    } else {
                        log::info!("Downloaded: {:.2} MB", downloaded as f64 / 1_048_576.0);
                    }
                }

                // Emit progress every 10 MB to reduce event spam
                let current_mb = downloaded / (10 * 1024 * 1024);
                if current_mb > last_emit_mb
                    || total_size.map_or(false, |total| downloaded >= total)
                {
                    last_emit_mb = current_mb;
                    let percentage =
                        total_size.map(|total| (downloaded as f64 / total as f64) * 100.0);
                    let message = if let Some(total) = total_size {
                        format!(
                            "Downloading llama.cpp: {:.2} MB / {:.2} MB",
                            downloaded as f64 / 1_048_576.0,
                            total as f64 / 1_048_576.0,
                        )
                    } else {
                        format!(
                            "Downloading llama.cpp: {:.2} MB",
                            downloaded as f64 / 1_048_576.0
                        )
                    };

                    // Update IPC state with progress
                    let _ = update_download_status(true, percentage);

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
            Some(Err(e)) => {
                consecutive_errors += 1;
                log::warn!(
                    "Chunk read error (attempt {}/{}): {}",
                    consecutive_errors,
                    MAX_CHUNK_RETRIES,
                    e
                );

                if consecutive_errors >= MAX_CHUNK_RETRIES {
                    return Err(format!(
                        "Failed to read chunk after {} retries: {}",
                        MAX_CHUNK_RETRIES, e
                    ));
                }

                if !supports_resume {
                    return Err(format!(
                        "Failed to read chunk and server does not support resume: {}",
                        e
                    ));
                }

                // Flush current data before reconnecting
                file.flush()
                    .await
                    .map_err(|e| format!("Failed to flush file before retry: {}", e))?;
                file.sync_all()
                    .await
                    .map_err(|e| format!("Failed to sync file before retry: {}", e))?;

                // Calculate backoff delay
                let delay = calculate_backoff_delay(consecutive_errors - 1);
                log::info!("Waiting {:?} before retry...", delay);

                let _ = app.emit(
                    "download-progress",
                    DownloadProgress {
                        downloaded,
                        total: total_size,
                        percentage: total_size
                            .map(|total| (downloaded as f64 / total as f64) * 100.0),
                        message: format!(
                            "Connection lost, retrying in {} seconds...",
                            delay.as_secs()
                        ),
                    },
                );

                tokio::time::sleep(delay).await;

                // Reconnect and resume from current position
                log::info!("Attempting to resume download from byte {}", downloaded);

                let (new_response, _) = start_download_request(&client, url, downloaded).await?;
                stream = new_response.bytes_stream();

                log::info!("Successfully resumed download");
            }
            None => {
                // Stream ended
                break;
            }
        }
    }

    log::info!(
        "Download completed! Total: {:.2} MB",
        downloaded as f64 / 1_048_576.0
    );

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


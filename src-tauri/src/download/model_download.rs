use super::download_utils::{load_config, verify_sha256};
use crate::ipc_state::update_download_status;
use crate::paths::{get_model_dir, is_model_downloaded};
use crate::types::{DownloadProgress, ModelInfo};
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

/// Create HTTP client for model downloads
fn create_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(600)) // 10 minutes for large models
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
        .map_err(|e| format!("Failed to download model: {}", e))?;

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

/// Download file with progress tracking, retry logic and resume support
async fn download_with_progress(
    url: &str,
    zip_path: &std::path::Path,
    model_name: &str,
    app: &AppHandle,
) -> Result<u64, String> {
    let client = create_http_client()?;

    log::info!("Downloading model '{}' from: {}", model_name, url);

    // Check if server supports range requests for resume capability
    let supports_resume = check_range_support(&client, url).await;

    // Check if partial download exists
    let mut downloaded: u64 = if supports_resume && zip_path.exists() {
        let existing_size = tokio::fs::metadata(zip_path)
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
        log::info!("Model size: {:.2} MB", size as f64 / 1_048_576.0);
    } else {
        log::warn!("Model size: unknown (no Content-Length header)");
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
            message: format!("Starting model '{}' download...", model_name),
        },
    );

    // Open file for writing (append if resuming)
    let mut file = if downloaded > 0 {
        let mut f = tokio::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(zip_path)
            .await
            .map_err(|e| format!("Failed to open zip file for resume: {}", e))?;
        // Seek to end to ensure we're appending
        f.seek(std::io::SeekFrom::End(0))
            .await
            .map_err(|e| format!("Failed to seek to end of file: {}", e))?;
        f
    } else {
        tokio::fs::File::create(zip_path)
            .await
            .map_err(|e| format!("Failed to create zip file: {}", e))?
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
                            "Downloading model '{}': {:.2} MB / {:.2} MB",
                            model_name,
                            downloaded as f64 / 1_048_576.0,
                            total as f64 / 1_048_576.0,
                        )
                    } else {
                        format!(
                            "Downloading model '{}': {:.2} MB",
                            model_name,
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

    log::info!("File synced successfully: {} bytes", downloaded);

    Ok(downloaded)
}

/// Extract model archive
fn extract_model_archive(
    zip_path: &std::path::Path,
    model_dir: &std::path::Path,
) -> Result<(), String> {
    let file =
        std::fs::File::open(zip_path).map_err(|e| format!("Failed to open zip file: {}", e))?;

    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    let archive_len = archive.len();
    log::info!("Archive contains {} files", archive_len);

    for i in 0..archive_len {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read file from archive: {}", e))?;

        let outpath = match file.enclosed_name() {
            Some(path) => model_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            log::info!("Creating directory: {}", file.name());
            fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            log::info!(
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

    log::info!("Extraction completed successfully!");
    Ok(())
}

/// Common download logic for models
async fn download_model_common(
    model_name: &str,
    model_url: &str,
    expected_sha256: &str,
    app: AppHandle,
) -> Result<String, String> {
    let model_dir = get_model_dir(model_name).map_err(|e| e.to_string())?;
    let zip_path = model_dir.join("model.zip");

    log::info!(
        "Starting model '{}' download from: {}",
        model_name, model_url
    );
    log::info!("Download destination: {:?}", zip_path);

    // Download with progress
    let downloaded = match download_with_progress(model_url, &zip_path, model_name, &app).await {
        Ok(size) => size,
        Err(e) => {
            // Clear IPC download status on error
            let _ = update_download_status(false, None);
            return Err(e);
        }
    };

    // Verify SHA-256 checksum
    if let Err(e) = verify_sha256(&zip_path, expected_sha256) {
        // Remove corrupted file
        fs::remove_file(&zip_path).ok();
        // Clear IPC download status on error
        let _ = update_download_status(false, None);
        return Err(format!("Model '{}' checksum verification failed: {}", model_name, e));
    }

    // Emit extraction progress
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            downloaded,
            total: Some(downloaded),
            percentage: Some(100.0),
            message: format!("Extracting model '{}'...", model_name),
        },
    );

    log::info!("Starting extraction...");

    // Extract archive
    if let Err(e) = extract_model_archive(&zip_path, &model_dir) {
        // Clear IPC download status on error
        let _ = update_download_status(false, None);
        return Err(e);
    }

    // Remove zip file
    log::info!("Removing temporary zip file...");
    fs::remove_file(&zip_path).ok();

    // Clear IPC download status on success
    let _ = update_download_status(false, None);

    log::info!("Model '{}' ready at: {:?}", model_name, model_dir);
    Ok(format!(
        "Model '{}' downloaded and extracted to: {:?}",
        model_name, model_dir
    ))
}

#[tauri::command]
pub async fn download_model_by_name(
    model_name: String,
    app: AppHandle,
) -> Result<String, String> {
    // Load config to get model URL and SHA-256
    let config = load_config()?;

    let model_config = config
        .models
        .get(&model_name)
        .ok_or_else(|| format!("Model '{}' not found in configuration", model_name))?;

    let model_url = &model_config.url;
    let expected_sha256 = &model_config.sha256;

    download_model_common(&model_name, model_url, expected_sha256, app).await
}


#[tauri::command]
pub async fn list_available_models() -> Result<Vec<ModelInfo>, String> {
    let config = load_config()?;
    let mut models = Vec::new();

    for (name, model_config) in config.models.iter() {
        let is_downloaded = is_model_downloaded(name).unwrap_or(false);
        let path = if is_downloaded {
            get_model_dir(name)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        };

        models.push(ModelInfo {
            name: name.clone(),
            version: model_config.version.clone(),
            is_downloaded,
            path,
        });
    }

    // Sort by name
    models.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(models)
}

#[tauri::command]
pub async fn delete_model(model_name: String) -> Result<String, String> {
    let model_dir = get_model_dir(&model_name).map_err(|e| e.to_string())?;

    if !model_dir.exists() {
        return Err(format!("Model '{}' is not downloaded", model_name));
    }

    fs::remove_dir_all(&model_dir)
        .map_err(|e| format!("Failed to delete model '{}': {}", model_name, e))?;

    Ok(format!("Model '{}' has been deleted", model_name))
}

#[tauri::command]
pub async fn check_model_downloaded(model_name: String) -> Result<bool, String> {
    is_model_downloaded(&model_name).map_err(|e| e.to_string())
}


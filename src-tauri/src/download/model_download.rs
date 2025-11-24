use super::download_utils::{load_config, verify_sha256};
use crate::paths::{get_model_dir, is_model_downloaded};
use crate::types::{DownloadProgress, ModelInfo};
use futures_util::StreamExt;
use std::fs;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

/// Create HTTP client for model downloads
fn create_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// Download file with progress tracking
async fn download_with_progress(
    url: &str,
    zip_path: &std::path::Path,
    model_name: &str,
    app: &AppHandle,
) -> Result<u64, String> {
    let client = create_http_client()?;

    let response = client
        .get(url)
        .header("Accept", "*/*")
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(|e| format!("Failed to download model: {}", e))?;

    let total_size = response.content_length();

    if let Some(size) = total_size {
        log::info!("Model size: {:.2} MB", size as f64 / 1_048_576.0);
    } else {
        log::info!("Model size: unknown");
    }

    // Emit initial progress
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            downloaded: 0,
            total: total_size,
            percentage: Some(0.0),
            message: format!("Starting model '{}' download...", model_name),
        },
    );

    let mut file = tokio::fs::File::create(zip_path)
        .await
        .map_err(|e| format!("Failed to create zip file: {}", e))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit_mb = 0u64;
    let mut last_log_mb = 0u64;

    log::info!("Starting download stream...");

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
        if current_mb > last_emit_mb || total_size.map_or(false, |total| downloaded >= total) {
            last_emit_mb = current_mb;
            let percentage = total_size.map(|total| (downloaded as f64 / total as f64) * 100.0);
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

    log::info!(
        "Download completed! Total: {:.2} MB",
        downloaded as f64 / 1_048_576.0
    );

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush file: {}", e))?;

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
    let downloaded = download_with_progress(model_url, &zip_path, model_name, &app).await?;

    // Verify SHA-256 checksum
    if let Err(e) = verify_sha256(&zip_path, expected_sha256) {
        // Remove corrupted file
        fs::remove_file(&zip_path).ok();
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
    extract_model_archive(&zip_path, &model_dir)?;

    // Remove zip file
    log::info!("Removing temporary zip file...");
    fs::remove_file(&zip_path).ok();

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


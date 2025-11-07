use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use sysinfo::System;

// Server state management
struct ServerState {
    process: Mutex<Option<Child>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerStatus {
    is_running: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct DownloadProgress {
    downloaded: u64,
    total: Option<u64>,
    percentage: Option<f64>,
    message: String,
}

// Get app data directory (cross-platform)
fn get_app_data_dir() -> Result<PathBuf> {
    let app_dir = dirs::data_dir()
        .ok_or_else(|| anyhow!("Failed to get data directory"))?
        .join("com.sigma-shield.app");
    
    fs::create_dir_all(&app_dir)?;
    Ok(app_dir)
}

// Get path to bin directory
fn get_bin_dir() -> Result<PathBuf> {
    let app_dir = get_app_data_dir()?;
    let bin_dir = app_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;
    Ok(bin_dir)
}

// Get path to llama.cpp binary
fn get_llama_binary_path() -> Result<PathBuf> {
    let bin_dir = get_bin_dir()?;
    let binary_path = bin_dir.join("llama-server");
    Ok(binary_path)
}

// Get path to model directory
fn get_model_dir() -> Result<PathBuf> {
    let app_dir = get_app_data_dir()?;
    let model_dir = app_dir.join("models");
    fs::create_dir_all(&model_dir)?;
    Ok(model_dir)
}

#[tauri::command]
async fn download_llama_cpp(app: AppHandle) -> Result<String, String> {
    let bin_dir = get_bin_dir().map_err(|e| e.to_string())?;
    let app_dir = get_app_data_dir().map_err(|e| e.to_string())?;
    
    // GitHub release URL for llama.cpp macOS Metal build
    // Using latest release - you may want to specify a version
    let url = "https://github.com/ggml-org/llama.cpp/releases/download/b6972/llama-b6972-bin-macos-arm64.zip";
    
    let binary_path = bin_dir.join("llama-server");
    
    // Check if already downloaded
    if binary_path.exists() {
        return Ok("llama.cpp already downloaded".to_string());
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
                message: format!("Downloading llama.cpp: {:.2} MB", downloaded as f64 / 1_048_576.0),
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
    let file = std::fs::File::open(&zip_path)
        .map_err(|e| format!("Failed to open zip file: {}", e))?;
    
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;
    
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
    
    Ok(format!("Downloaded llama.cpp to: {:?}", binary_path))
}

#[tauri::command]
async fn download_model(model_url: String, app: AppHandle) -> Result<String, String> {
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
    let response = client.get(&model_url)
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
                println!("Downloaded: {:.2} MB ({:.1}%)", downloaded as f64 / 1_048_576.0, pct);
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
                format!("Downloading model: {:.2} MB", downloaded as f64 / 1_048_576.0)
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
    
    println!("Download completed! Total: {:.2} MB", downloaded as f64 / 1_048_576.0);
    
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
    let file = std::fs::File::open(&zip_path)
        .map_err(|e| format!("Failed to open zip file: {}", e))?;
    
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;
    
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
            println!("Extracting file {}/{}: {} ({:.2} MB)", 
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
    Ok(format!("Model downloaded and extracted to: {:?}", model_dir))
}

#[tauri::command]
async fn start_server(
    state: State<'_, ServerState>,
    port: u16,
) -> Result<String, String> {
    let mut process_guard = state.process.lock().unwrap();
    
    // Check if server is already running
    if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(None) => return Err("Server is already running".to_string()),
            Ok(Some(_)) => {
                *process_guard = None;
            }
            Err(_) => {
                *process_guard = None;
            }
        }
    }
    
    let binary_path = get_llama_binary_path().map_err(|e| e.to_string())?;
    let model_dir = get_model_dir().map_err(|e| e.to_string())?;
    let model_path = model_dir.join("model.gguf");
    
    // Check if binary exists
    if !binary_path.exists() {
        return Err("llama.cpp not found. Please download it first.".to_string());
    }
    
    // Check if model exists
    if !model_path.exists() {
        return Err("Model not found. Please download it first.".to_string());
    }
    
    // Start llama-server in API-only mode (no web frontend)
    let child = Command::new(&binary_path)
        .arg("-m")
        .arg(&model_path)
        .arg("--port")
        .arg(port.to_string())
        .arg("--ctx-size")
        .arg("30000")
        .arg("--n-gpu-layers")
        .arg("41")
        .spawn()
        .map_err(|e| format!("Failed to start server: {}", e))?;
    
    *process_guard = Some(child);
    
    Ok(format!("Server started on port {}", port))
}

#[tauri::command]
async fn stop_server(state: State<'_, ServerState>) -> Result<String, String> {
    let mut process_guard = state.process.lock().unwrap();
    
    if let Some(mut child) = process_guard.take() {
        child
            .kill()
            .map_err(|e| format!("Failed to stop server: {}", e))?;
        child
            .wait()
            .map_err(|e| format!("Failed to wait for server: {}", e))?;
        Ok("Server stopped".to_string())
    } else {
        Err("LLM is not running".to_string())
    }
}

#[tauri::command]
async fn get_server_status(state: State<'_, ServerState>) -> Result<ServerStatus, String> {
    let mut process_guard = state.process.lock().unwrap();
    
    if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(None) => Ok(ServerStatus {
                is_running: true,
                message: "LLM is running".to_string(),
            }),
            Ok(Some(status)) => {
                *process_guard = None;
                Ok(ServerStatus {
                    is_running: false,
                    message: format!("LLM exited with status: {}", status),
                })
            }
            Err(e) => {
                *process_guard = None;
                Ok(ServerStatus {
                    is_running: false,
                    message: format!("Failed to check LLM status: {}", e),
                })
            }
        }
    } else {
        Ok(ServerStatus {
            is_running: false,
            message: "LLM is not running".to_string(),
        })
    }
}

#[tauri::command]
fn get_app_data_path() -> Result<String, String> {
    get_app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_system_memory_gb() -> Result<u64, String> {
    let mut sys = System::new_all();
    sys.refresh_memory();
    
    let total_memory_bytes = sys.total_memory();
    let total_memory_gb = total_memory_bytes / (1024 * 1024 * 1024);
    
    Ok(total_memory_gb)
}

#[tauri::command]
async fn clear_binaries(state: State<'_, ServerState>) -> Result<String, String> {
    // Stop server if running
    let mut process_guard = state.process.lock().unwrap();
    if let Some(mut child) = process_guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    drop(process_guard);
    
    let bin_dir = get_bin_dir().map_err(|e| e.to_string())?;
    
    if bin_dir.exists() {
        fs::remove_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to remove bin directory: {}", e))?;
        println!("Removed bin directory: {:?}", bin_dir);
    }
    
    Ok("Binaries cleared successfully".to_string())
}

#[tauri::command]
async fn clear_models() -> Result<String, String> {
    let model_dir = get_model_dir().map_err(|e| e.to_string())?;
    
    if model_dir.exists() {
        fs::remove_dir_all(&model_dir)
            .map_err(|e| format!("Failed to remove models directory: {}", e))?;
        println!("Removed models directory: {:?}", model_dir);
    }
    
    Ok("Models cleared successfully".to_string())
}

#[tauri::command]
async fn clear_all_data(state: State<'_, ServerState>) -> Result<String, String> {
    // Stop server if running
    let mut process_guard = state.process.lock().unwrap();
    if let Some(mut child) = process_guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    drop(process_guard);
    
    let app_dir = get_app_data_dir().map_err(|e| e.to_string())?;
    
    if app_dir.exists() {
        fs::remove_dir_all(&app_dir)
            .map_err(|e| format!("Failed to remove app data directory: {}", e))?;
        println!("Removed app data directory: {:?}", app_dir);
    }
    
    Ok("All data cleared successfully".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(ServerState {
            process: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            download_llama_cpp,
            download_model,
            start_server,
            stop_server,
            get_server_status,
            get_app_data_path,
            get_system_memory_gb,
            clear_binaries,
            clear_models,
            clear_all_data,
        ])
        .on_window_event(|window, event| {
            // Stop llama-server when window is closing
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = window.try_state::<ServerState>() {
                    let mut process_guard = state.process.lock().unwrap();
                    if let Some(mut child) = process_guard.take() {
                        println!("Stopping llama-server on window close...");
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

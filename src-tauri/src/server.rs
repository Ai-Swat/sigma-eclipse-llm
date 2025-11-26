use crate::paths::{get_llama_binary_path, get_model_file_path, get_short_path};
use crate::settings::get_active_model;
use crate::types::{ServerState, ServerStatus};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::State;

#[tauri::command]
pub async fn start_server(
    state: State<'_, ServerState>,
    port: u16,
    ctx_size: u32,
    gpu_layers: u32,
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

    // Validate ctx_size (8k to 100k)
    if ctx_size < 6000 || ctx_size > 100000 {
        return Err("Context size must be between 8000 and 100000".to_string());
    }

    // Validate gpu_layers (0 to 41)
    if gpu_layers > 41 {
        return Err("GPU layers must be between 0 and 41".to_string());
    }

    let binary_path = get_llama_binary_path().map_err(|e| e.to_string())?;
    
    // Get active model from settings
    let active_model = get_active_model().map_err(|e| e.to_string())?;
    let model_path = get_model_file_path(&active_model).map_err(|e| e.to_string())?;

    // Check if binary exists
    if !binary_path.exists() {
        return Err("llama.cpp not found. Please download it first.".to_string());
    }

    // Check if model exists
    if !model_path.exists() {
        return Err(format!("Model '{}' not found. Please download it first.", active_model));
    }

    // Convert paths to short format on Windows to handle Cyrillic characters
    let binary_path_safe = get_short_path(&binary_path).map_err(|e| e.to_string())?;
    let model_path_safe = get_short_path(&model_path).map_err(|e| e.to_string())?;

    log::info!("Starting llama-server with binary: {:?}", binary_path_safe);
    log::info!("Using model: {:?}", model_path_safe);

    // Start llama-server in API-only mode (no web frontend)
    // Use kill_on_drop to ensure process is killed when parent exits
    let mut command = Command::new(&binary_path_safe);
    command
        .arg("-m")
        .arg(&model_path_safe)
        .arg("--port")
        .arg(port.to_string())
        .arg("--ctx-size")
        .arg(ctx_size.to_string())
        .arg("--n-gpu-layers")
        .arg(gpu_layers.to_string())
        .arg("--flash-attn").arg("auto")
        .arg("--batch-size").arg("2048")
        .arg("--ubatch-size").arg("512")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // On Unix, create a new process group so we can kill the entire group
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    
    // On Windows, hide console window and add CUDA paths
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
        
      }
    
    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to start server: {}", e))?;

    // Capture stdout and stderr for logging
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    log::info!("[llama.cpp] {}", line);
                }
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    log::warn!("[llama.cpp] {}", line);
                }
            }
        });
    }

    *process_guard = Some(child);

    Ok(format!(
        "Server started on port {} (model: {}, ctx: {}, gpu layers: {})",
        port, active_model, ctx_size, gpu_layers
    ))
}

#[tauri::command]
pub async fn stop_server(state: State<'_, ServerState>) -> Result<String, String> {
    let mut process_guard = state.process.lock().unwrap();

    if let Some(mut child) = process_guard.take() {
        // On Unix, kill the entire process group
        #[cfg(unix)]
        {
            let pid = child.id() as i32;
            // Kill the process group (negative PID means process group)
            unsafe {
                libc::kill(-pid, libc::SIGTERM);
                // Wait a bit for graceful shutdown
                std::thread::sleep(std::time::Duration::from_millis(100));
                // Force kill if still running
                libc::kill(-pid, libc::SIGKILL);
            }
        }
        
        // Standard kill for non-Unix or as fallback
        let _ = child.kill();
        let _ = child.wait();
        
        Ok("Server stopped".to_string())
    } else {
        Err("LLM is not running".to_string())
    }
}

#[tauri::command]
pub async fn get_server_status(state: State<'_, ServerState>) -> Result<ServerStatus, String> {
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


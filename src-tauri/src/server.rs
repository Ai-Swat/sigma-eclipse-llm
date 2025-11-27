use crate::ipc_state::update_server_status;
use crate::server_manager::{get_status, start_server_process, stop_server_by_pid, ServerConfig};
use crate::types::{ServerState, ServerStatus};
use std::io::{BufRead, BufReader};
use tauri::State;

#[tauri::command]
pub async fn start_server(
    state: State<'_, ServerState>,
    port: u16,
    ctx_size: u32,
    gpu_layers: u32,
) -> Result<String, String> {
    let mut process_guard = state.process.lock().unwrap();

    // Check if local process is running
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

    // Use shared server manager to start process
    let config = ServerConfig {
        port,
        ctx_size,
        gpu_layers,
    };

    let mut child = start_server_process(config, true).map_err(|e| e.to_string())?;
    let pid = child.id();

    // Capture stdout and stderr for logging in Tauri context
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
        "Server started on port {} (PID: {}, ctx: {}, gpu layers: {})",
        port, pid, ctx_size, gpu_layers
    ))
}

#[tauri::command]
pub async fn stop_server(state: State<'_, ServerState>) -> Result<String, String> {
    let mut process_guard = state.process.lock().unwrap();

    if let Some(mut child) = process_guard.take() {
        let pid = child.id();
        
        // Use shared server manager to stop
        stop_server_by_pid(pid).map_err(|e| e.to_string())?;
        
        // Also clean up local Child handle
        let _ = child.kill();
        let _ = child.wait();
        
        Ok("Server stopped".to_string())
    } else {
        // Check if server is running elsewhere (e.g., via Native Host)
        if let Ok((is_running, Some(pid))) = get_status() {
            if is_running {
                stop_server_by_pid(pid).map_err(|e| e.to_string())?;
                return Ok(format!("Server stopped (PID: {})", pid));
            }
        }
        
        Err("LLM is not running".to_string())
    }
}

#[tauri::command]
pub async fn get_server_status(state: State<'_, ServerState>) -> Result<ServerStatus, String> {
    let mut process_guard = state.process.lock().unwrap();

    // First check local process
    if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(None) => {
                return Ok(ServerStatus {
                    is_running: true,
                    message: "LLM is running".to_string(),
                });
            }
            Ok(Some(status)) => {
                *process_guard = None;
                // Update IPC state
                let _ = update_server_status(false, None);
                return Ok(ServerStatus {
                    is_running: false,
                    message: format!("LLM exited with status: {}", status),
                });
            }
            Err(e) => {
                *process_guard = None;
                // Update IPC state
                let _ = update_server_status(false, None);
                return Ok(ServerStatus {
                    is_running: false,
                    message: format!("Failed to check LLM status: {}", e),
                });
            }
        }
    }

    // Check shared IPC state (may be running via Native Host)
    match get_status() {
        Ok((is_running, pid)) => Ok(ServerStatus {
            is_running,
            message: if is_running {
                format!("LLM is running (PID: {})", pid.unwrap_or(0))
            } else {
                "LLM is not running".to_string()
            },
        }),
        Err(e) => Ok(ServerStatus {
            is_running: false,
            message: format!("Failed to check status: {}", e),
        }),
    }
}


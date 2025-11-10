use crate::paths::{get_llama_binary_path, get_model_dir};
use crate::types::{ServerState, ServerStatus};
use std::process::Command;
use tauri::State;

#[tauri::command]
pub async fn start_server(state: State<'_, ServerState>, port: u16) -> Result<String, String> {
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
pub async fn stop_server(state: State<'_, ServerState>) -> Result<String, String> {
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


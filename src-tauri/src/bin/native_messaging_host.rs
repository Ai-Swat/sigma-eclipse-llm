// Native Messaging Host for Browser Extension Communication
// Implements Chrome Native Messaging Protocol
// https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::process::Child;
use std::sync::Mutex;

// Import shared modules from main crate
use sigma_eclipse_lib::ipc_state::{is_tauri_app_running, read_ipc_state};
use sigma_eclipse_lib::server_manager::{
    check_server_running, get_status, start_server_process, stop_server_by_pid, ServerConfig,
};

/// Global state for server process
/// Note: This is process-local, shared state is in ipc_state.json
static SERVER_PROCESS: Mutex<Option<Child>> = Mutex::new(None);

#[derive(Debug, Deserialize)]
struct NativeMessage {
    id: String,
    command: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct NativeResponse {
    id: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Read a message from stdin using Native Messaging Protocol
/// Format: [4 bytes length][JSON message]
fn read_message() -> Result<NativeMessage> {
    let mut length_bytes = [0u8; 4];
    io::stdin()
        .read_exact(&mut length_bytes)
        .context("Failed to read message length")?;

    let length = u32::from_ne_bytes(length_bytes) as usize;

    let mut buffer = vec![0u8; length];
    io::stdin()
        .read_exact(&mut buffer)
        .context("Failed to read message body")?;

    let message: NativeMessage =
        serde_json::from_slice(&buffer).context("Failed to parse message JSON")?;

    Ok(message)
}

/// Send a response to stdout using Native Messaging Protocol
/// Format: [4 bytes length][JSON message]
fn send_response(response: &NativeResponse) -> Result<()> {
    let json = serde_json::to_string(response).context("Failed to serialize response")?;
    let length = json.len() as u32;

    io::stdout()
        .write_all(&length.to_ne_bytes())
        .context("Failed to write response length")?;
    io::stdout()
        .write_all(json.as_bytes())
        .context("Failed to write response body")?;
    io::stdout().flush().context("Failed to flush stdout")?;

    Ok(())
}

/// Log to stderr (stdout is reserved for Native Messaging Protocol)
macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("[Native Host] {}", format!($($arg)*));
    };
}

/// Handle start_server command
fn handle_start_server(params: Value) -> Result<Value> {
    let port = params["port"].as_u64().unwrap_or(8080) as u16;
    let ctx_size = params["ctx_size"].as_u64().unwrap_or(8192) as u32;
    let gpu_layers = params["gpu_layers"].as_u64().unwrap_or(0) as u32;

    log!("Starting server: port={}, ctx_size={}, gpu_layers={}", port, ctx_size, gpu_layers);

    // Use shared server manager
    let config = ServerConfig {
        port,
        ctx_size,
        gpu_layers,
    };

    let child = start_server_process(config, false)?;
    let pid = child.id();

    log!("Server started with PID: {}", pid);

    // Store process handle locally
    let mut process_guard = SERVER_PROCESS.lock().unwrap();
    *process_guard = Some(child);

    Ok(json!({
        "message": format!("Server started on port {} (PID: {})", port, pid),
        "pid": pid,
        "port": port,
    }))
}

/// Handle stop_server command
fn handle_stop_server() -> Result<Value> {
    log!("Stopping server");

    let mut process_guard = SERVER_PROCESS.lock().unwrap();

    if let Some(mut child) = process_guard.take() {
        let pid = child.id();

        // Use shared server manager
        stop_server_by_pid(pid)?;

        // Also clean up local Child handle
        let _ = child.kill();
        let _ = child.wait();

        log!("Server stopped (PID: {})", pid);

        Ok(json!({
            "message": "Server stopped",
        }))
    } else {
        // Check if server is running elsewhere (e.g., via Tauri)
        if let Some(pid) = check_server_running()? {
            stop_server_by_pid(pid)?;
            return Ok(json!({
                "message": format!("Server stopped (PID: {})", pid),
            }));
        }

        Err(anyhow::anyhow!("Server is not running"))
    }
}

/// Handle get_server_status command
fn handle_get_server_status() -> Result<Value> {
    // Use shared server manager
    let (is_running, pid) = get_status()?;
    
    // Get additional info from IPC state
    let state = read_ipc_state()?;

    log!("Server status: running={}", is_running);

    Ok(json!({
        "is_running": is_running,
        "pid": pid,
        "port": state.server_port,
        "ctx_size": state.server_ctx_size,
        "gpu_layers": state.server_gpu_layers,
        "message": if is_running { "Server is running" } else { "Server is not running" },
    }))
}

/// Handle isDownloading command
fn handle_is_downloading() -> Result<Value> {
    let state = read_ipc_state()?;

    log!("Download status: downloading={}, progress={:?}", 
        state.is_downloading, state.download_progress);

    Ok(json!({
        "is_downloading": state.is_downloading,
        "progress": state.download_progress,
    }))
}

/// Handle get_app_status command - check if Tauri app is running
fn handle_get_app_status() -> Result<Value> {
    let is_running = is_tauri_app_running()?;
    let state = read_ipc_state()?;

    log!("App status: running={}, pid={:?}", is_running, state.tauri_app_pid);

    Ok(json!({
        "is_running": is_running,
        "pid": state.tauri_app_pid,
        "last_heartbeat": state.tauri_app_heartbeat,
        "message": if is_running { "App is running" } else { "App is not running" },
    }))
}

/// Handle launch_app command - launch Tauri app if not running
fn handle_launch_app() -> Result<Value> {
    // Check if already running
    if is_tauri_app_running()? {
        log!("App is already running");
        return Ok(json!({
            "launched": false,
            "message": "App is already running",
        }));
    }

    log!("Launching Sigma Eclipse app...");

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // Try to launch via bundle identifier first
        let result = Command::new("open")
            .args(["-b", "com.sigma-eclipse.llm"])
            .spawn();
        
        match result {
            Ok(_) => {
                log!("App launched successfully via bundle ID");
                return Ok(json!({
                    "launched": true,
                    "message": "App launched successfully",
                }));
            }
            Err(e) => {
                log!("Failed to launch via bundle ID: {}", e);
            }
        }
        
        // Fallback: try to launch by app name
        let result = Command::new("open")
            .args(["-a", "Sigma Eclipse LLM"])
            .spawn();
        
        match result {
            Ok(_) => {
                log!("App launched successfully via app name");
                return Ok(json!({
                    "launched": true,
                    "message": "App launched successfully",
                }));
            }
            Err(e) => {
                log!("Failed to launch via app name: {}", e);
                return Err(anyhow::anyhow!("Failed to launch app: {}", e));
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        
        // Try to find and launch the app from common locations
        let possible_paths = [
            dirs::data_local_dir()
                .map(|p| p.join("Programs").join("Sigma Eclipse LLM").join("Sigma Eclipse LLM.exe")),
            dirs::home_dir()
                .map(|p| p.join("AppData").join("Local").join("Programs").join("Sigma Eclipse LLM").join("Sigma Eclipse LLM.exe")),
        ];
        
        for path_opt in possible_paths.iter() {
            if let Some(path) = path_opt {
                if path.exists() {
                    match Command::new(path).spawn() {
                        Ok(_) => {
                            log!("App launched successfully from {:?}", path);
                            return Ok(json!({
                                "launched": true,
                                "message": "App launched successfully",
                            }));
                        }
                        Err(e) => {
                            log!("Failed to launch from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        
        return Err(anyhow::anyhow!("Could not find Sigma Eclipse LLM executable"));
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        
        // Try common Linux app locations
        let possible_commands = [
            "sigma-eclipse-llm",
            "/usr/bin/sigma-eclipse-llm",
            "/usr/local/bin/sigma-eclipse-llm",
        ];
        
        for cmd in possible_commands {
            match Command::new(cmd).spawn() {
                Ok(_) => {
                    log!("App launched successfully via {}", cmd);
                    return Ok(json!({
                        "launched": true,
                        "message": "App launched successfully",
                    }));
                }
                Err(_) => continue,
            }
        }
        
        return Err(anyhow::anyhow!("Could not find Sigma Eclipse LLM executable"));
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        return Err(anyhow::anyhow!("Platform not supported"));
    }
}

/// Process a single command
fn process_command(message: NativeMessage) -> NativeResponse {
    log!("Received command: {} (id: {})", message.command, message.id);

    let result = match message.command.as_str() {
        "start_server" => handle_start_server(message.params),
        "stop_server" => handle_stop_server(),
        "get_server_status" => handle_get_server_status(),
        "isDownloading" => handle_is_downloading(),
        "get_app_status" => handle_get_app_status(),
        "launch_app" => handle_launch_app(),
        _ => Err(anyhow::anyhow!("Unknown command: {}", message.command)),
    };

    match result {
        Ok(data) => NativeResponse {
            id: message.id,
            success: true,
            data: Some(data),
            error: None,
        },
        Err(e) => {
            log!("Error: {}", e);
            NativeResponse {
                id: message.id,
                success: false,
                data: None,
                error: Some(e.to_string()),
            }
        }
    }
}

fn main() {
    log!("Native Messaging Host started");
    log!("Protocol: Chrome Native Messaging");
    log!("App: Sigma Eclipse LLM");

    // Main message loop
    loop {
        match read_message() {
            Ok(message) => {
                let response = process_command(message);
                if let Err(e) = send_response(&response) {
                    log!("Failed to send response: {}", e);
                    break;
                }
            }
            Err(e) => {
                log!("Failed to read message: {}", e);
                break;
            }
        }
    }

    log!("Native Messaging Host stopped");
}


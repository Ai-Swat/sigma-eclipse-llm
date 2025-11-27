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
use sigma_shield_lib::ipc_state::read_ipc_state;
use sigma_shield_lib::server_manager::{
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

/// Process a single command
fn process_command(message: NativeMessage) -> NativeResponse {
    log!("Received command: {} (id: {})", message.command, message.id);

    let result = match message.command.as_str() {
        "start_server" => handle_start_server(message.params),
        "stop_server" => handle_stop_server(),
        "get_server_status" => handle_get_server_status(),
        "isDownloading" => handle_is_downloading(),
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
    log!("App: Sigma Shield LLM");

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


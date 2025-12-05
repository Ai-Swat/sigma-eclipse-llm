// Native Messaging Host for Browser Extension Communication
// Implements Chrome Native Messaging Protocol
// https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::Child;
use std::sync::Mutex;

// Import shared modules from main crate
use sigma_eclipse_lib::ipc_state::{is_tauri_app_running, read_ipc_state};
use sigma_eclipse_lib::server_manager::{
    check_server_running, get_status, start_server_process, stop_server_by_pid, ServerConfig,
};
use sigma_eclipse_lib::settings::get_server_settings;

/// Global state for server process
/// Note: This is process-local, shared state is in ipc_state.json
static SERVER_PROCESS: Mutex<Option<Child>> = Mutex::new(None);

/// Global log file handle
static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

/// Cached status for change detection (checked after each message)
static CACHED_STATUS: Mutex<Option<CachedStatus>> = Mutex::new(None);

/// Set binary mode for stdin/stdout on Windows
/// This is critical for Native Messaging Protocol to work correctly
#[cfg(windows)]
fn set_binary_mode() {
    // Windows _setmode constants
    const O_BINARY: i32 = 0x8000;
    
    extern "C" {
        fn _setmode(fd: i32, mode: i32) -> i32;
    }
    
    unsafe {
        // stdin = 0, stdout = 1 (standard C file descriptors)
        _setmode(0, O_BINARY);
        _setmode(1, O_BINARY);
    }
}

#[cfg(not(windows))]
fn set_binary_mode() {
    // No-op on non-Windows platforms
}

/// Get path to log file
fn get_log_file_path() -> Option<PathBuf> {
    let app_dir = dirs::data_dir()?.join("com.sigma-eclipse.llm");
    std::fs::create_dir_all(&app_dir).ok()?;
    Some(app_dir.join("native-host.log"))
}

/// Initialize log file (overwrites on each start)
fn init_log_file() {
    if let Some(path) = get_log_file_path() {
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)  // Overwrite file on each start
            .open(&path)
        {
            let mut guard = LOG_FILE.lock().unwrap();
            *guard = Some(file);
        }
    }
}

/// Write to log file
fn write_to_log_file(message: &str) {
    let mut guard = LOG_FILE.lock().unwrap();
    if let Some(ref mut file) = *guard {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] {}", timestamp, message);
        let _ = file.flush();
    }
}

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

#[derive(Debug, Serialize)]
struct StatusPushMessage {
    #[serde(rename = "type")]
    msg_type: &'static str,
    data: Value,
}

/// Cached status for change detection
#[derive(Default, Clone, PartialEq)]
struct CachedStatus {
    app_running: bool,
    model_running: bool,
    is_downloading: bool,
    download_progress: Option<f64>,
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

/// Send a push message to stdout (same protocol as response)
fn send_push(message: &StatusPushMessage) -> Result<()> {
    let json = serde_json::to_string(message).context("Failed to serialize push")?;
    let length = json.len() as u32;

    io::stdout()
        .write_all(&length.to_ne_bytes())
        .context("Failed to write push length")?;
    io::stdout()
        .write_all(json.as_bytes())
        .context("Failed to write push body")?;
    io::stdout().flush().context("Failed to flush stdout")?;

    Ok(())
}

/// Log to stderr and file (stdout is reserved for Native Messaging Protocol)
macro_rules! log {
    ($($arg:tt)*) => {
        let msg = format!($($arg)*);
        eprintln!("[Native Host] {}", msg);
        write_to_log_file(&msg);
    };
}

/// Check current status and send push if changed
fn check_and_push_status() {
    log!("check_and_push_status called");
    
    let new_status = CachedStatus {
        app_running: is_tauri_app_running().unwrap_or(false),
        model_running: get_status().map(|(r, _)| r).unwrap_or(false),
        is_downloading: read_ipc_state().map(|s| s.is_downloading).unwrap_or(false),
        download_progress: read_ipc_state().ok().and_then(|s| s.download_progress),
    };

    log!(
        "new_status: app={}, model={}, downloading={}, progress={:?}",
        new_status.app_running,
        new_status.model_running,
        new_status.is_downloading,
        new_status.download_progress
    );

    let mut cached_guard = CACHED_STATUS.lock().unwrap();
    let should_push = match &*cached_guard {
        Some(cached) => *cached != new_status,
        None => true, // First check, always send initial status
    };

    log!("should_push: {}", should_push);

    if should_push {
        log!("Status changed, sending push update");

        let push = StatusPushMessage {
            msg_type: "status_update",
            data: json!({
                "appRunning": new_status.app_running,
                "modelRunning": new_status.model_running,
                "isDownloading": new_status.is_downloading,
                "downloadProgress": new_status.download_progress,
            }),
        };

        if let Err(e) = send_push(&push) {
            log!("Failed to send push: {}", e);
        } else {
            log!("Push sent successfully");
        }

        *cached_guard = Some(new_status);
    }
}

/// Handle start_server command
fn handle_start_server() -> Result<Value> {
    // Get settings from settings.json
    let (port, ctx_size, gpu_layers) = get_server_settings()?;

    // Use shared server manager
    let config = ServerConfig {
        port,
        ctx_size,
        gpu_layers,
    };

    let child = start_server_process(config, false)?;
    let pid = child.id();

    log!("Server started: port={}, pid={}", port, pid);

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
    let mut process_guard = SERVER_PROCESS.lock().unwrap();

    if let Some(mut child) = process_guard.take() {
        let pid = child.id();

        // Use shared server manager
        stop_server_by_pid(pid)?;

        // Also clean up local Child handle
        let _ = child.kill();
        let _ = child.wait();

        log!("Server stopped: pid={}", pid);

        Ok(json!({
            "message": "Server stopped",
        }))
    } else {
        // Check if server is running elsewhere (e.g., via Tauri)
        if let Some(pid) = check_server_running()? {
            stop_server_by_pid(pid)?;
            log!("Server stopped: pid={}", pid);
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

    Ok(json!({
        "is_downloading": state.is_downloading,
        "progress": state.download_progress,
    }))
}

/// Handle get_app_status command - check if Tauri app is running
fn handle_get_app_status() -> Result<Value> {
    let is_running = is_tauri_app_running()?;
    let state = read_ipc_state()?;

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
        return Ok(json!({
            "launched": false,
            "message": "App is already running",
        }));
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // Try to launch via bundle identifier first
        if Command::new("open")
            .args(["-b", "com.sigma-eclipse.llm"])
            .spawn()
            .is_ok()
        {
            log!("App launched");
            return Ok(json!({
                "launched": true,
                "message": "App launched successfully",
            }));
        }
        
        // Fallback: try to launch by app name
        if Command::new("open")
            .args(["-a", "Sigma Eclipse LLM"])
            .spawn()
            .is_ok()
        {
            log!("App launched");
            return Ok(json!({
                "launched": true,
                "message": "App launched successfully",
            }));
        }
        
        return Err(anyhow::anyhow!("Failed to launch app"));
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        
        // Try to find and launch the app from common locations
        // NSIS installer may put the app in different locations
        let possible_paths = [
            // Direct in AppData\Local (NSIS default for per-user install)
            dirs::data_local_dir()
                .map(|p| p.join("Sigma Eclipse LLM").join("sigma-eclipse.exe")),
            // In Programs subfolder
            dirs::data_local_dir()
                .map(|p| p.join("Programs").join("Sigma Eclipse LLM").join("sigma-eclipse.exe")),
            // Explicit path via home dir
            dirs::home_dir()
                .map(|p| p.join("AppData").join("Local").join("Sigma Eclipse LLM").join("sigma-eclipse.exe")),
            dirs::home_dir()
                .map(|p| p.join("AppData").join("Local").join("Programs").join("Sigma Eclipse LLM").join("sigma-eclipse.exe")),
        ];
        
        for path_opt in possible_paths.iter() {
            if let Some(path) = path_opt {
                log!("Checking path: {:?}", path);
                if path.exists() {
                    if Command::new(path)
                        .creation_flags(CREATE_NO_WINDOW)
                        .spawn()
                        .is_ok() 
                    {
                        log!("App launched from: {:?}", path);
                        return Ok(json!({
                            "launched": true,
                            "message": "App launched successfully",
                        }));
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
            if Command::new(cmd).spawn().is_ok() {
                log!("App launched");
                return Ok(json!({
                    "launched": true,
                    "message": "App launched successfully",
                }));
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
    let result = match message.command.as_str() {
        "start_server" => handle_start_server(),
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
            log!("Error: {} (cmd: {})", e, message.command);
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
    // Set binary mode for stdin/stdout on Windows (critical for Native Messaging!)
    set_binary_mode();
    
    // Initialize log file (overwrites previous)
    init_log_file();
    log!("Host started");

    // Main message loop
    loop {
        match read_message() {
            Ok(message) => {
                let response = process_command(message);
                if send_response(&response).is_err() {
                    break;
                }
                // Check and send status push after each processed message
                check_and_push_status();
            }
            Err(e) => {
                log!("read_error: {}", e);
                break;
            }
        }
    }

    log!("Host stopped");
}


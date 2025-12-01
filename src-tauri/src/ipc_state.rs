// Shared state management for IPC between Native Messaging Host and Tauri app
// Uses file-based state storage for cross-process communication

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// IPC State stored in a JSON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcState {
    /// Server process ID if running
    pub server_pid: Option<u32>,
    /// Is server running
    pub server_running: bool,
    /// Is download in progress
    pub is_downloading: bool,
    /// Current download progress percentage
    pub download_progress: Option<f64>,
    /// Server port
    pub server_port: Option<u16>,
    /// Server context size
    pub server_ctx_size: Option<u32>,
    /// Server GPU layers
    pub server_gpu_layers: Option<u32>,
    /// Tauri app process ID if running
    pub tauri_app_pid: Option<u32>,
    /// Tauri app last heartbeat timestamp (Unix timestamp in seconds)
    pub tauri_app_heartbeat: Option<u64>,
}

impl Default for IpcState {
    fn default() -> Self {
        Self {
            server_pid: None,
            server_running: false,
            is_downloading: false,
            download_progress: None,
            server_port: None,
            server_ctx_size: None,
            server_gpu_layers: None,
            tauri_app_pid: None,
            tauri_app_heartbeat: None,
        }
    }
}

/// Get path to IPC state file
pub fn get_ipc_state_path() -> Result<PathBuf> {
    let app_data = dirs::data_dir()
        .context("Failed to get app data directory")?
        .join("com.sigma-eclipse.llm");
    
    fs::create_dir_all(&app_data)?;
    Ok(app_data.join("ipc_state.json"))
}

/// Read IPC state from file
pub fn read_ipc_state() -> Result<IpcState> {
    let path = get_ipc_state_path()?;
    
    if !path.exists() {
        return Ok(IpcState::default());
    }
    
    let contents = fs::read_to_string(&path)
        .context("Failed to read IPC state file")?;
    
    let state: IpcState = serde_json::from_str(&contents)
        .unwrap_or_default();
    
    Ok(state)
}

/// Write IPC state to file
pub fn write_ipc_state(state: &IpcState) -> Result<()> {
    let path = get_ipc_state_path()?;
    let contents = serde_json::to_string_pretty(state)
        .context("Failed to serialize IPC state")?;
    
    fs::write(&path, contents)
        .context("Failed to write IPC state file")?;
    
    Ok(())
}

/// Update server status in IPC state
pub fn update_server_status(running: bool, pid: Option<u32>) -> Result<()> {
    let mut state = read_ipc_state()?;
    state.server_running = running;
    state.server_pid = pid;
    write_ipc_state(&state)?;
    Ok(())
}

/// Update download status in IPC state
pub fn update_download_status(is_downloading: bool, progress: Option<f64>) -> Result<()> {
    let mut state = read_ipc_state()?;
    state.is_downloading = is_downloading;
    state.download_progress = progress;
    write_ipc_state(&state)?;
    Ok(())
}

/// Check if process is actually running (cross-platform)
pub fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .contains(&pid.to_string())
            })
            .unwrap_or(false)
    }
}

/// Heartbeat timeout in seconds (if no heartbeat for this long, app is considered dead)
pub const HEARTBEAT_TIMEOUT_SECS: u64 = 10;

/// Get current Unix timestamp in seconds
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Update Tauri app heartbeat (called periodically by Tauri app)
pub fn update_tauri_app_heartbeat(pid: u32) -> Result<()> {
    let mut state = read_ipc_state()?;
    state.tauri_app_pid = Some(pid);
    state.tauri_app_heartbeat = Some(current_timestamp());
    write_ipc_state(&state)?;
    Ok(())
}

/// Clear Tauri app status (called when Tauri app exits)
pub fn clear_tauri_app_status() -> Result<()> {
    let mut state = read_ipc_state()?;
    state.tauri_app_pid = None;
    state.tauri_app_heartbeat = None;
    write_ipc_state(&state)?;
    Ok(())
}

/// Check if Tauri app is running based on heartbeat and PID
pub fn is_tauri_app_running() -> Result<bool> {
    let state = read_ipc_state()?;
    
    // Check if we have PID and heartbeat
    let (pid, heartbeat) = match (state.tauri_app_pid, state.tauri_app_heartbeat) {
        (Some(pid), Some(hb)) => (pid, hb),
        _ => return Ok(false),
    };
    
    // Check if heartbeat is recent
    let now = current_timestamp();
    if now.saturating_sub(heartbeat) > HEARTBEAT_TIMEOUT_SECS {
        return Ok(false);
    }
    
    // Verify process is actually running
    Ok(is_process_running(pid))
}


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
        }
    }
}

/// Get path to IPC state file
pub fn get_ipc_state_path() -> Result<PathBuf> {
    let app_data = dirs::data_dir()
        .context("Failed to get app data directory")?
        .join("sigma-eclipse");
    
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
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .contains(&pid.to_string())
            })
            .unwrap_or(false)
    }
}


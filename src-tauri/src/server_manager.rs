// Shared server management logic
// Used by both Tauri commands and Native Messaging Host

use crate::ipc_state::{is_process_running, read_ipc_state, update_server_status};
use crate::paths::{get_llama_binary_path, get_model_file_path, get_short_path};
use crate::settings::get_active_model;
use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};

/// Configuration for starting the server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub ctx_size: u32,
    pub gpu_layers: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 10345,
            ctx_size: 8192,
            gpu_layers: 0,
        }
    }
}

/// Validate server configuration
pub fn validate_config(config: &ServerConfig) -> Result<()> {
    if config.ctx_size < 6000 || config.ctx_size > 100000 {
        anyhow::bail!("Context size must be between 6000 and 100000");
    }

    if config.gpu_layers > 41 {
        anyhow::bail!("GPU layers must be between 0 and 41");
    }

    Ok(())
}

/// Check if server is already running via IPC state
pub fn check_server_running() -> Result<Option<u32>> {
    let state = read_ipc_state()?;
    
    if state.server_running {
        if let Some(pid) = state.server_pid {
            if is_process_running(pid) {
                return Ok(Some(pid));
            }
            // Process is stale, clean up
            update_server_status(false, None)?;
        }
    }
    
    Ok(None)
}

/// Start the llama-server process
pub fn start_server_process(
    config: ServerConfig,
    capture_output: bool,
) -> Result<Child> {
    // Validate configuration
    validate_config(&config)?;

    // Check if already running
    if let Some(pid) = check_server_running()? {
        anyhow::bail!("Server is already running (PID: {})", pid);
    }

    let binary_path = get_llama_binary_path().context("Failed to get binary path")?;
    let active_model = get_active_model().context("Failed to get active model")?;
    let model_path = get_model_file_path(&active_model).context("Failed to get model path")?;

    // Check if binary exists
    if !binary_path.exists() {
        anyhow::bail!("llama.cpp not found. Please download it first.");
    }

    // Check if model exists
    if !model_path.exists() {
        anyhow::bail!("Model '{}' not found. Please download it first.", active_model);
    }

    // Convert paths to short format on Windows to handle Cyrillic characters
    let binary_path_safe = get_short_path(&binary_path).context("Failed to get short path for binary")?;
    let model_path_safe = get_short_path(&model_path).context("Failed to get short path for model")?;

    log::info!("Starting llama-server with binary: {:?}", binary_path_safe);
    log::info!("Using model: {:?}", model_path_safe);
    log::info!("Config: port={}, ctx_size={}, gpu_layers={}", 
        config.port, config.ctx_size, config.gpu_layers);

    // Build command
    let mut command = Command::new(&binary_path_safe);
    command
        .arg("-m")
        .arg(&model_path_safe)
        .arg("--port")
        .arg(config.port.to_string())
        .arg("--ctx-size")
        .arg(config.ctx_size.to_string())
        .arg("--n-gpu-layers")
        .arg(config.gpu_layers.to_string())
        .arg("--flash-attn")
        .arg("auto")
        .arg("--batch-size")
        .arg("2048")
        .arg("--ubatch-size")
        .arg("512");

    // Configure stdio
    if capture_output {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }

    // On Unix, create a new process group
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    // On Windows, hide console window
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    // Spawn process
    let child = command.spawn().context("Failed to start server process")?;
    let pid = child.id();

    log::info!("Server started with PID: {}", pid);

    // Update IPC state
    update_server_status(true, Some(pid))?;

    // Update config in IPC state
    let mut state = read_ipc_state()?;
    state.server_port = Some(config.port);
    state.server_ctx_size = Some(config.ctx_size);
    state.server_gpu_layers = Some(config.gpu_layers);
    crate::ipc_state::write_ipc_state(&state)?;

    Ok(child)
}

/// Stop the server by PID
pub fn stop_server_by_pid(pid: u32) -> Result<()> {
    log::info!("Stopping server (PID: {})", pid);

    #[cfg(unix)]
    {
        let pid_i32 = pid as i32;
        unsafe {
            // Try graceful shutdown first
            libc::kill(-pid_i32, libc::SIGTERM);
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Force kill if still running
            libc::kill(-pid_i32, libc::SIGKILL);
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
    }

    // Update IPC state
    update_server_status(false, None)?;

    // Clear config
    let mut state = read_ipc_state()?;
    state.server_port = None;
    state.server_ctx_size = None;
    state.server_gpu_layers = None;
    crate::ipc_state::write_ipc_state(&state)?;

    log::info!("Server stopped");

    Ok(())
}

/// Get current server status from IPC state
pub fn get_status() -> Result<(bool, Option<u32>)> {
    let state = read_ipc_state()?;

    let is_running = if state.server_running {
        if let Some(pid) = state.server_pid {
            is_process_running(pid)
        } else {
            false
        }
    } else {
        false
    };

    // Update state if stale
    if state.server_running && !is_running {
        update_server_status(false, None)?;
    }

    Ok((is_running, state.server_pid))
}


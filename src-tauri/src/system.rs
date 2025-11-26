use crate::paths::{get_app_data_dir, get_bin_dir, get_models_root_dir};
use crate::types::{RecommendedSettings, ServerState};
use std::fs;
use sysinfo::System;
use tauri::State;

#[tauri::command]
pub fn get_app_data_path() -> Result<String, String> {
    get_app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_logs_path() -> Result<String, String> {
    get_app_data_dir()
        .map(|p| p.join("logs").to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_system_memory_gb() -> Result<u64, String> {
    let mut sys = System::new_all();
    sys.refresh_memory();

    let total_memory_bytes = sys.total_memory();
    let total_memory_gb = total_memory_bytes / (1024 * 1024 * 1024);

    Ok(total_memory_gb)
}

// ============================================================================
// GPU Detection (Windows only)
// ============================================================================

#[cfg(target_os = "windows")]
#[derive(Debug)]
struct GpuInfo {
    has_nvidia: bool,
    vram_gb: u64,
    is_10xx_series: bool,
}

#[cfg(target_os = "windows")]
impl Default for GpuInfo {
    fn default() -> Self {
        Self {
            has_nvidia: false,
            vram_gb: 0,
            is_10xx_series: false,
        }
    }
}

#[cfg(target_os = "windows")]
fn detect_10xx_series(output: &str) -> bool {
    let lower = output.to_lowercase();
    // GTX 10XX series: GTX 1050, 1060, 1070, 1080, etc.
    lower.contains("gtx 10") 
        || lower.contains("geforce gtx 10")
        || lower.contains("gtx105")  // Without space
        || lower.contains("gtx106")
        || lower.contains("gtx107")
        || lower.contains("gtx108")
}

#[cfg(target_os = "windows")]
fn parse_vram_from_wmic(output_str: &str) -> Option<u64> {
    // wmic output format: "AdapterRAM  Name"
    // Example: "8589934592  NVIDIA GeForce GTX 1070"
    for line in output_str.lines() {
        if line.contains("nvidia") || line.contains("NVIDIA") {
            // Split line and find first numeric value
            let parts: Vec<&str> = line.split_whitespace().collect();
            
            // First element should be AdapterRAM if it's a valid number
            if let Some(&first_part) = parts.first() {
                if let Ok(ram_bytes) = first_part.parse::<u64>() {
                    // Only consider values that make sense as RAM (> 500MB in bytes)
                    // This filters out model numbers like "1070"
                    if ram_bytes > 500_000_000 {
                        let vram_gb = ram_bytes / (1024 * 1024 * 1024);
                        log::info!("Parsed VRAM from wmic: {}GB ({} bytes)", vram_gb, ram_bytes);
                        return Some(vram_gb);
                    }
                }
            }
        }
    }
    log::warn!("Failed to parse VRAM from wmic output");
    None
}

#[cfg(target_os = "windows")]
fn try_detect_via_wmic() -> Option<GpuInfo> {
    use std::process::Command;

    let output = Command::new("wmic")
        .args(&["path", "win32_VideoController", "get", "name,AdapterRAM"])
        .output()
        .ok()?;

    let output_str = String::from_utf8(output.stdout).ok()?;
    let lower_output = output_str.to_lowercase();

    if !lower_output.contains("nvidia") {
        return None;
    }

    let gpu_info = GpuInfo {
        has_nvidia: true,
        is_10xx_series: detect_10xx_series(&output_str),
        vram_gb: parse_vram_from_wmic(&output_str).unwrap_or(0),
    };

    Some(gpu_info)
}

#[cfg(target_os = "windows")]
fn try_detect_vram_via_nvidia_smi() -> Option<u64> {
    use std::process::Command;

    let output = Command::new("nvidia-smi")
        .args(&[
            "--query-gpu=memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    let output_str = String::from_utf8(output.stdout).ok()?;
    let vram_mb = output_str.trim().parse::<u64>().ok()?;
    Some(vram_mb / 1024)
}

#[cfg(target_os = "windows")]
fn detect_nvidia_gpu() -> GpuInfo {
    let mut gpu_info = try_detect_via_wmic().unwrap_or_default();

    if let Some(vram) = try_detect_vram_via_nvidia_smi() {
        if !gpu_info.has_nvidia {
            gpu_info.has_nvidia = true;
        }

        if vram > 0 && (gpu_info.vram_gb == 0 || vram > gpu_info.vram_gb) {
            if gpu_info.vram_gb > 0 && gpu_info.vram_gb != vram {
                log::info!(
                    "nvidia-smi VRAM override: {}GB -> {}GB",
                    gpu_info.vram_gb,
                    vram
                );
            }
            gpu_info.vram_gb = vram;
        }
    } else if gpu_info.has_nvidia && gpu_info.vram_gb == 0 {
        log::warn!("Detected Nvidia GPU but failed to determine VRAM via wmic or nvidia-smi");
    }

    log::info!(
        "GPU detection: has_nvidia={}, vram={}GB, is_10xx={}",
        gpu_info.has_nvidia,
        gpu_info.vram_gb,
        gpu_info.is_10xx_series
    );

    gpu_info
}

// ============================================================================
// Settings Calculation Helpers
// ============================================================================

fn calculate_ctx_size_by_ram(memory_gb: u64) -> u32 {
    if memory_gb < 16 {
        6000
    } else if memory_gb < 24 {
        12000
    } else {
        28000
    }
}

// ============================================================================
// Platform-specific Settings Logic
// ============================================================================

#[cfg(target_os = "macos")]
fn get_platform_settings(memory_gb: u64) -> (String, u32) {
    let model = if memory_gb < 16 {
        "model_s".to_string()
    } else {
        "model".to_string()
    };
    let ctx = calculate_ctx_size_by_ram(memory_gb);
    
    log::info!(
        "[macOS] Settings: RAM={}GB, model={}, ctx={}",
        memory_gb, model, ctx
    );
    
    (model, ctx)
}

#[cfg(target_os = "windows")]
fn get_platform_settings(memory_gb: u64) -> (String, u32) {
    let gpu_info = detect_nvidia_gpu();

    let (model, ctx) = if !gpu_info.has_nvidia {
        // No Nvidia GPU - use model_s with RAM-based settings
        ("model_s".to_string(), calculate_ctx_size_by_ram(memory_gb))
    } else if gpu_info.is_10xx_series {
        // Nvidia 10XX series - always ctx 6000 regardless of VRAM
        let model = if gpu_info.vram_gb < 7 {
            "model_s".to_string()
        } else {
            "model".to_string()
        };
        (model, 12000)
    } else if gpu_info.vram_gb < 7 {
        // Nvidia GPU (non-10XX) with less than 8GB VRAM
        ("model_s".to_string(), calculate_ctx_size_by_ram(memory_gb))
    } else {
        // Nvidia GPU (non-10XX) with 8GB+ VRAM
        ("model".to_string(), calculate_ctx_size_by_ram(memory_gb))
    };

    log::info!(
        "[Windows] Settings: RAM={}GB, GPU={}/{}GB/10xx={}, model={}, ctx={}",
        memory_gb,
        if gpu_info.has_nvidia { "Nvidia" } else { "None" },
        gpu_info.vram_gb,
        gpu_info.is_10xx_series,
        model,
        ctx
    );

    (model, ctx)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn get_platform_settings(memory_gb: u64) -> (String, u32) {
    let model = if memory_gb < 15 {
        "model_s".to_string()
    } else {
        "model".to_string()
    };
    let ctx = calculate_ctx_size_by_ram(memory_gb);
    
    log::info!(
        "[Other OS] Settings: RAM={}GB, model={}, ctx={}",
        memory_gb, model, ctx
    );
    
    (model, ctx)
}

// ============================================================================
// Main Settings Command
// ============================================================================

#[tauri::command]
pub fn get_recommended_settings() -> Result<RecommendedSettings, String> {
    let memory_gb = get_system_memory_gb()?;
    let (recommended_model, recommended_ctx_size) = get_platform_settings(memory_gb);
    let recommended_gpu_layers = 41;

    Ok(RecommendedSettings {
        memory_gb,
        recommended_model,
        recommended_ctx_size,
        recommended_gpu_layers,
    })
}

// ============================================================================
// Process Management Helpers
// ============================================================================

fn stop_server_process(state: &State<'_, ServerState>) {
    let mut process_guard = state.process.lock().unwrap();
    if let Some(mut child) = process_guard.take() {
        // On Unix, kill the entire process group
        #[cfg(unix)]
        {
            let pid = child.id() as i32;
            unsafe {
                libc::kill(-pid, libc::SIGTERM);
                std::thread::sleep(std::time::Duration::from_millis(100));
                libc::kill(-pid, libc::SIGKILL);
            }
        }

        let _ = child.kill();
        let _ = child.wait();
    }
}

// ============================================================================
// Clear Data Commands
// ============================================================================

#[tauri::command]
pub async fn clear_binaries(state: State<'_, ServerState>) -> Result<String, String> {
    stop_server_process(&state);

    let bin_dir = get_bin_dir().map_err(|e| e.to_string())?;

    if bin_dir.exists() {
        fs::remove_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to remove bin directory: {}", e))?;
        log::info!("Removed bin directory: {:?}", bin_dir);
    }

    Ok("Binaries cleared successfully".to_string())
}

#[tauri::command]
pub async fn clear_models() -> Result<String, String> {
    let models_dir = get_models_root_dir().map_err(|e| e.to_string())?;

    if models_dir.exists() {
        fs::remove_dir_all(&models_dir)
            .map_err(|e| format!("Failed to remove models directory: {}", e))?;
        log::info!("Removed models directory: {:?}", models_dir);
    }

    Ok("Models cleared successfully".to_string())
}

#[tauri::command]
pub async fn clear_all_data(state: State<'_, ServerState>) -> Result<String, String> {
    stop_server_process(&state);

    let app_dir = get_app_data_dir().map_err(|e| e.to_string())?;

    if app_dir.exists() {
        fs::remove_dir_all(&app_dir)
            .map_err(|e| format!("Failed to remove app data directory: {}", e))?;
        log::info!("Removed app data directory: {:?}", app_dir);
    }

    Ok("All data cleared successfully".to_string())
}

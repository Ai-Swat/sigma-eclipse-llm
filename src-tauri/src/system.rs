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
pub fn get_system_memory_gb() -> Result<u64, String> {
    let mut sys = System::new_all();
    sys.refresh_memory();

    let total_memory_bytes = sys.total_memory();
    let total_memory_gb = total_memory_bytes / (1024 * 1024 * 1024);

    Ok(total_memory_gb)
}

#[tauri::command]
pub fn get_recommended_settings() -> Result<RecommendedSettings, String> {
    let memory_gb = get_system_memory_gb()?;

    // Determine recommended model based on RAM
    let recommended_model = if memory_gb < 16 {
        "model_s".to_string() // Smaller model for systems with < 16GB RAM
    } else {
        "model".to_string() // Full model for systems with >= 16GB RAM
    };

    // Determine recommended context size based on RAM
    let recommended_ctx_size = if memory_gb < 16 {
        6000 // 6k context for low RAM systems
    } else if memory_gb >= 16 && memory_gb < 24 {
        15000 // 15k context for medium RAM systems
    } else {
        30000 // 30k context for high RAM systems
    };

    // GPU layers - default to 41 (all layers on GPU if available)
    let recommended_gpu_layers = 41;

    println!(
        "Recommended settings: RAM={}GB, model={}, ctx={}, gpu_layers={}",
        memory_gb, recommended_model, recommended_ctx_size, recommended_gpu_layers
    );

    Ok(RecommendedSettings {
        memory_gb,
        recommended_model,
        recommended_ctx_size,
        recommended_gpu_layers,
    })
}

#[tauri::command]
pub async fn clear_binaries(state: State<'_, ServerState>) -> Result<String, String> {
    // Stop server if running
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
    drop(process_guard);

    let bin_dir = get_bin_dir().map_err(|e| e.to_string())?;

    if bin_dir.exists() {
        fs::remove_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to remove bin directory: {}", e))?;
        println!("Removed bin directory: {:?}", bin_dir);
    }

    Ok("Binaries cleared successfully".to_string())
}

#[tauri::command]
pub async fn clear_models() -> Result<String, String> {
    let models_dir = get_models_root_dir().map_err(|e| e.to_string())?;

    if models_dir.exists() {
        fs::remove_dir_all(&models_dir)
            .map_err(|e| format!("Failed to remove models directory: {}", e))?;
        println!("Removed models directory: {:?}", models_dir);
    }

    Ok("Models cleared successfully".to_string())
}

#[tauri::command]
pub async fn clear_all_data(state: State<'_, ServerState>) -> Result<String, String> {
    // Stop server if running
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
    drop(process_guard);

    let app_dir = get_app_data_dir().map_err(|e| e.to_string())?;

    if app_dir.exists() {
        fs::remove_dir_all(&app_dir)
            .map_err(|e| format!("Failed to remove app data directory: {}", e))?;
        println!("Removed app data directory: {:?}", app_dir);
    }

    Ok("All data cleared successfully".to_string())
}


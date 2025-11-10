use crate::paths::{get_app_data_dir, get_bin_dir, get_model_dir};
use crate::types::ServerState;
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
pub async fn clear_binaries(state: State<'_, ServerState>) -> Result<String, String> {
    // Stop server if running
    let mut process_guard = state.process.lock().unwrap();
    if let Some(mut child) = process_guard.take() {
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
    let model_dir = get_model_dir().map_err(|e| e.to_string())?;

    if model_dir.exists() {
        fs::remove_dir_all(&model_dir)
            .map_err(|e| format!("Failed to remove models directory: {}", e))?;
        println!("Removed models directory: {:?}", model_dir);
    }

    Ok("Models cleared successfully".to_string())
}

#[tauri::command]
pub async fn clear_all_data(state: State<'_, ServerState>) -> Result<String, String> {
    // Stop server if running
    let mut process_guard = state.process.lock().unwrap();
    if let Some(mut child) = process_guard.take() {
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


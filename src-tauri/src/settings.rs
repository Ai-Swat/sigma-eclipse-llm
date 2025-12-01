use crate::paths::get_app_data_dir;
use crate::system::calculate_recommended_settings;
use crate::types::AppSettings;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Get path to settings file
fn get_settings_path() -> Result<PathBuf> {
    let app_dir = get_app_data_dir()?;
    Ok(app_dir.join("settings.json"))
}

/// Create default settings based on system recommended values
fn create_default_settings() -> AppSettings {
    match calculate_recommended_settings() {
        Ok(recommended) => {
            log::info!(
                "Creating default settings from recommended: model={}, ctx_size={}, gpu_layers={}",
                recommended.recommended_model,
                recommended.recommended_ctx_size,
                recommended.recommended_gpu_layers
            );
            AppSettings {
                active_model: recommended.recommended_model,
                port: 10345,
                ctx_size: recommended.recommended_ctx_size,
                gpu_layers: recommended.recommended_gpu_layers,
            }
        }
        Err(e) => {
            log::warn!("Failed to get recommended settings, using hardcoded defaults: {}", e);
            AppSettings::default()
        }
    }
}

/// Load settings from settings.json
pub fn load_settings() -> Result<AppSettings> {
    let settings_path = get_settings_path()?;
    
    if !settings_path.exists() {
        // Create default settings based on system recommendations
        let settings = create_default_settings();
        // Save them so they persist
        save_settings(&settings)?;
        return Ok(settings);
    }
    
    let content = fs::read_to_string(&settings_path)?;
    let settings: AppSettings = serde_json::from_str(&content)?;
    
    Ok(settings)
}

/// Save settings to settings.json
pub fn save_settings(settings: &AppSettings) -> Result<()> {
    let settings_path = get_settings_path()?;
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(&settings_path, content)?;
    
    Ok(())
}

/// Get active model name from settings
pub fn get_active_model() -> Result<String> {
    let settings = load_settings()?;
    Ok(settings.active_model)
}

/// Set active model in settings
pub fn set_active_model(model_name: String) -> Result<()> {
    let mut settings = load_settings()?;
    settings.active_model = model_name;
    save_settings(&settings)?;
    
    Ok(())
}

/// Get server settings (port, ctx_size, gpu_layers)
pub fn get_server_settings() -> Result<(u16, u32, u32)> {
    let settings = load_settings()?;
    Ok((settings.port, settings.ctx_size, settings.gpu_layers))
}

/// Set server port
pub fn set_port(port: u16) -> Result<()> {
    let mut settings = load_settings()?;
    settings.port = port;
    save_settings(&settings)?;
    Ok(())
}

/// Set context size
pub fn set_ctx_size(ctx_size: u32) -> Result<()> {
    let mut settings = load_settings()?;
    settings.ctx_size = ctx_size;
    save_settings(&settings)?;
    Ok(())
}

/// Set GPU layers
pub fn set_gpu_layers(gpu_layers: u32) -> Result<()> {
    let mut settings = load_settings()?;
    settings.gpu_layers = gpu_layers;
    save_settings(&settings)?;
    Ok(())
}

// Tauri commands

#[tauri::command]
pub async fn get_active_model_command() -> Result<String, String> {
    get_active_model().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_active_model_command(model_name: String) -> Result<String, String> {
    set_active_model(model_name.clone()).map_err(|e| e.to_string())?;
    Ok(format!("Active model set to: {}", model_name))
}

#[tauri::command]
pub async fn get_settings_command() -> Result<AppSettings, String> {
    load_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_port_command(port: u16) -> Result<String, String> {
    set_port(port).map_err(|e| e.to_string())?;
    Ok(format!("Port set to: {}", port))
}

#[tauri::command]
pub async fn set_ctx_size_command(ctx_size: u32) -> Result<String, String> {
    set_ctx_size(ctx_size).map_err(|e| e.to_string())?;
    Ok(format!("Context size set to: {}", ctx_size))
}

#[tauri::command]
pub async fn set_gpu_layers_command(gpu_layers: u32) -> Result<String, String> {
    set_gpu_layers(gpu_layers).map_err(|e| e.to_string())?;
    Ok(format!("GPU layers set to: {}", gpu_layers))
}


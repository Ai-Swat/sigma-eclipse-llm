use crate::paths::get_app_data_dir;
use crate::types::AppSettings;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Get path to settings file
fn get_settings_path() -> Result<PathBuf> {
    let app_dir = get_app_data_dir()?;
    Ok(app_dir.join("settings.json"))
}

/// Load settings from settings.json
pub fn load_settings() -> Result<AppSettings> {
    let settings_path = get_settings_path()?;
    
    if !settings_path.exists() {
        // Return default settings if file doesn't exist
        return Ok(AppSettings::default());
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


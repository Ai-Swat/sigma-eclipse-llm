use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;

// Get app data directory (cross-platform)
pub fn get_app_data_dir() -> Result<PathBuf> {
    let app_dir = dirs::data_dir()
        .ok_or_else(|| anyhow!("Failed to get data directory"))?
        .join("com.sigma-shield.llm");

    fs::create_dir_all(&app_dir)?;
    Ok(app_dir)
}

// Get path to bin directory
pub fn get_bin_dir() -> Result<PathBuf> {
    let app_dir = get_app_data_dir()?;
    let bin_dir = app_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;
    Ok(bin_dir)
}

// Get path to llama.cpp binary
pub fn get_llama_binary_path() -> Result<PathBuf> {
    let bin_dir = get_bin_dir()?;
    let binary_path = bin_dir.join("llama-server");
    Ok(binary_path)
}

// Get path to model directory
pub fn get_model_dir() -> Result<PathBuf> {
    let app_dir = get_app_data_dir()?;
    let model_dir = app_dir.join("models");
    fs::create_dir_all(&model_dir)?;
    Ok(model_dir)
}


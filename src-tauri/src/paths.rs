use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

/// Convert path to Windows short path format (8.3) to handle Cyrillic characters
/// This is necessary because llama.cpp uses C functions that don't handle UTF-8 paths well on Windows
#[cfg(target_os = "windows")]
pub fn get_short_path(long_path: &PathBuf) -> Result<PathBuf> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::Storage::FileSystem::GetShortPathNameW;
    
    // Convert PathBuf to wide string (UTF-16)
    let wide: Vec<u16> = long_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    
    let mut buffer = vec![0u16; MAX_PATH as usize];
    
    // Call Windows API to get short path
    let result = unsafe {
        GetShortPathNameW(
            PWSTR(wide.as_ptr() as *mut u16),
            Some(&mut buffer),
        )
    };
    
    if result == 0 {
        // If GetShortPathNameW fails, return original path as fallback
        log::warn!("Failed to get short path for {:?}, using original path", long_path);
        return Ok(long_path.clone());
    }
    
    // Convert result back to String
    let short_path_str = String::from_utf16_lossy(&buffer[..result as usize]);
    Ok(PathBuf::from(short_path_str))
}

#[cfg(not(target_os = "windows"))]
pub fn get_short_path(long_path: &PathBuf) -> Result<PathBuf> {
    // On non-Windows platforms, just return the original path
    Ok(long_path.clone())
}

// Get app data directory (cross-platform)
pub fn get_app_data_dir() -> Result<PathBuf> {
    let app_dir = dirs::data_dir()
        .ok_or_else(|| anyhow!("Failed to get data directory"))?
        .join("com.sigma-eclipse.llm");

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
    
    #[cfg(target_os = "windows")]
    let binary_path = bin_dir.join("llama-server.exe");
    
    #[cfg(not(target_os = "windows"))]
    let binary_path = bin_dir.join("llama-server");
    
    Ok(binary_path)
}

// Get path to models root directory
pub fn get_models_root_dir() -> Result<PathBuf> {
    let app_dir = get_app_data_dir()?;
    let models_dir = app_dir.join("models");
    fs::create_dir_all(&models_dir)?;
    Ok(models_dir)
}

// Get path to specific model directory
pub fn get_model_dir(model_name: &str) -> Result<PathBuf> {
    let models_root = get_models_root_dir()?;
    let model_dir = models_root.join(model_name);
    fs::create_dir_all(&model_dir)?;
    Ok(model_dir)
}

// Get path to model file (.gguf)
pub fn get_model_file_path(model_name: &str) -> Result<PathBuf> {
    let model_dir = get_model_dir(model_name)?;
    
    // Look for any .gguf file in the model directory
    if let Ok(entries) = fs::read_dir(&model_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("gguf") {
                return Ok(path);
            }
        }
    }
    
    // Fallback: if no .gguf found, return default name
    Ok(model_dir.join("model.gguf"))
}

// Check if model is downloaded
pub fn is_model_downloaded(model_name: &str) -> Result<bool> {
    let model_dir = get_model_dir(model_name)?;
    
    // Check if directory exists and has .gguf file
    if !model_dir.exists() {
        return Ok(false);
    }
    
    // Look for any .gguf file in the directory
    if let Ok(entries) = fs::read_dir(&model_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("gguf") {
                return Ok(true);
            }
        }
    }
    
    Ok(false)
}


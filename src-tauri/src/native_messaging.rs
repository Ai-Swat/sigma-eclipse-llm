// Native Messaging Host manifest installation
// Automatically installs the manifest for Sigma browser extension

use anyhow::{Context, Result};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// Extension ID for the Sigma Eclipse browser extension (loaded from .env at build time)
const EXTENSION_ID: &str = env!("EXTENSION_ID");

/// Native messaging host name
const HOST_NAME: &str = "com.sigma_eclipse.host";

/// Get the path to the native messaging host binary inside the app bundle
#[cfg(target_os = "macos")]
fn get_host_binary_path() -> Result<PathBuf> {
    // Get the path to the current executable
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    
    // The binary should be in the same directory (Contents/MacOS/)
    let macos_dir = exe_path
        .parent()
        .context("Failed to get MacOS directory")?;
    
    let host_path = macos_dir.join("sigma-eclipse-host");
    
    if host_path.exists() {
        Ok(host_path)
    } else {
        // Fallback: check if running in development mode
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("sigma-eclipse-host");
        
        if dev_path.exists() {
            Ok(dev_path)
        } else {
            anyhow::bail!("Native messaging host binary not found")
        }
    }
}

/// Get the path to the native messaging host binary on Windows
#[cfg(target_os = "windows")]
fn get_host_binary_path() -> Result<PathBuf> {
    // Get the path to the current executable
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    
    // The host binary should be in the same directory as the main executable
    let exe_dir = exe_path
        .parent()
        .context("Failed to get executable directory")?;
    
    let host_path = exe_dir.join("sigma-eclipse-host.exe");
    
    if host_path.exists() {
        Ok(host_path)
    } else {
        // Fallback: check if running in development mode
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("sigma-eclipse-host.exe");
        
        if dev_path.exists() {
            Ok(dev_path)
        } else {
            anyhow::bail!("Native messaging host binary not found")
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn get_host_binary_path() -> Result<PathBuf> {
    anyhow::bail!("Native messaging installation not yet supported on this platform")
}

/// Get the Sigma browser Native Messaging Hosts directory for the current user
#[cfg(target_os = "macos")]
fn get_sigma_native_hosts_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to get home directory")?;
    Ok(home
        .join("Library")
        .join("Application Support")
        .join("Sigma")
        .join("NativeMessagingHosts"))
}

/// Get the directory where manifest file will be stored on Windows
/// Note: On Windows, the manifest file path is registered in Windows Registry
#[cfg(target_os = "windows")]
fn get_sigma_native_hosts_dir() -> Result<PathBuf> {
    let app_data = dirs::data_local_dir()
        .context("Failed to get local app data directory")?;
    Ok(app_data
        .join("Sigma")
        .join("NativeMessagingHosts"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn get_sigma_native_hosts_dir() -> Result<PathBuf> {
    anyhow::bail!("Not supported on this platform")
}

/// Generate the manifest JSON content
fn generate_manifest(host_binary_path: &PathBuf) -> String {
    let manifest = json!({
        "name": HOST_NAME,
        "description": "Sigma Eclipse LLM Native Messaging Host",
        "path": host_binary_path.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": [
            format!("chrome-extension://{}/", EXTENSION_ID)
        ]
    });
    
    serde_json::to_string_pretty(&manifest).unwrap()
}

/// Install the native messaging manifest for a specific browser (macOS/Linux)
#[cfg(not(target_os = "windows"))]
fn install_manifest_for_browser(hosts_dir: &PathBuf, host_binary_path: &PathBuf) -> Result<()> {
    // Create the directory if it doesn't exist
    fs::create_dir_all(hosts_dir)
        .with_context(|| format!("Failed to create directory: {:?}", hosts_dir))?;
    
    // Generate manifest content
    let manifest_content = generate_manifest(host_binary_path);
    
    // Write the manifest file
    let manifest_path = hosts_dir.join(format!("{}.json", HOST_NAME));
    fs::write(&manifest_path, &manifest_content)
        .with_context(|| format!("Failed to write manifest: {:?}", manifest_path))?;
    
    log::info!("Installed native messaging manifest: {:?}", manifest_path);
    
    Ok(())
}

/// Install the native messaging manifest for Windows
/// On Windows, we need to:
/// 1. Write the manifest JSON file
/// 2. Register the manifest path in Windows Registry (multiple browser paths)
#[cfg(target_os = "windows")]
fn install_manifest_for_browser(hosts_dir: &PathBuf, host_binary_path: &PathBuf) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;
    
    // Create the directory if it doesn't exist
    fs::create_dir_all(hosts_dir)
        .with_context(|| format!("Failed to create directory: {:?}", hosts_dir))?;
    
    // Generate manifest content
    let manifest_content = generate_manifest(host_binary_path);
    
    // Write the manifest file
    let manifest_path = hosts_dir.join(format!("{}.json", HOST_NAME));
    fs::write(&manifest_path, &manifest_content)
        .with_context(|| format!("Failed to write manifest: {:?}", manifest_path))?;
    
    log::info!("Installed native messaging manifest file: {:?}", manifest_path);
    
    let manifest_path_str = manifest_path.to_string_lossy().to_string();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    
    // Registry paths for different browsers
    // Sigma browser may use Chrome's path or its own path
    let registry_paths = [
        format!("Software\\Sigma\\NativeMessagingHosts\\{}", HOST_NAME),
        format!("Software\\Google\\Chrome\\NativeMessagingHosts\\{}", HOST_NAME),
    ];
    
    for registry_path in &registry_paths {
        match hkcu.create_subkey(registry_path) {
            Ok((key, _)) => {
                if let Err(e) = key.set_value("", &manifest_path_str) {
                    log::warn!("Failed to set registry value for {}: {}", registry_path, e);
                } else {
                    log::info!("Registered native messaging host in registry: {} -> {}", registry_path, manifest_path_str);
                }
            }
            Err(e) => {
                log::warn!("Failed to create registry key {}: {}", registry_path, e);
            }
        }
    }
    
    Ok(())
}

/// Install native messaging manifests for Sigma browser
pub fn install_native_messaging_manifests() -> Result<()> {
    log::info!("Installing native messaging manifests...");
    
    let host_binary_path = get_host_binary_path()?;
    log::info!("Host binary path: {:?}", host_binary_path);
    
    // Verify the binary exists and is executable
    if !host_binary_path.exists() {
        anyhow::bail!("Host binary not found at {:?}", host_binary_path);
    }
    
    // Install for Sigma browser
    match get_sigma_native_hosts_dir() {
        Ok(sigma_dir) => {
            if let Err(e) = install_manifest_for_browser(&sigma_dir, &host_binary_path) {
                log::warn!("Failed to install Sigma browser manifest: {}", e);
            }
        }
        Err(e) => {
            log::warn!("Sigma browser not supported: {}", e);
        }
    }
    
    log::info!("Native messaging manifests installation complete");
    
    Ok(())
}

/// Check if native messaging is properly configured (macOS/Linux)
#[cfg(not(target_os = "windows"))]
pub fn check_native_messaging_status() -> Result<NativeMessagingStatus> {
    let host_binary_path = get_host_binary_path().ok();
    let host_exists = host_binary_path.as_ref().map(|p| p.exists()).unwrap_or(false);
    
    let sigma_manifest_exists = get_sigma_native_hosts_dir()
        .map(|dir| dir.join(format!("{}.json", HOST_NAME)).exists())
        .unwrap_or(false);
    
    Ok(NativeMessagingStatus {
        host_binary_path,
        host_exists,
        sigma_manifest_installed: sigma_manifest_exists,
    })
}

/// Check if native messaging is properly configured (Windows)
#[cfg(target_os = "windows")]
pub fn check_native_messaging_status() -> Result<NativeMessagingStatus> {
    use winreg::enums::*;
    use winreg::RegKey;
    
    let host_binary_path = get_host_binary_path().ok();
    let host_exists = host_binary_path.as_ref().map(|p| p.exists()).unwrap_or(false);
    
    // Check if manifest file exists
    let manifest_file_exists = get_sigma_native_hosts_dir()
        .map(|dir| dir.join(format!("{}.json", HOST_NAME)).exists())
        .unwrap_or(false);
    
    // Check if any registry key exists (Sigma or Chrome)
    let registry_exists = {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let sigma_path = format!("Software\\Sigma\\NativeMessagingHosts\\{}", HOST_NAME);
        let chrome_path = format!("Software\\Google\\Chrome\\NativeMessagingHosts\\{}", HOST_NAME);
        hkcu.open_subkey(&sigma_path).is_ok() || hkcu.open_subkey(&chrome_path).is_ok()
    };
    
    // Both file and at least one registry entry must exist for proper installation
    let sigma_manifest_installed = manifest_file_exists && registry_exists;
    
    Ok(NativeMessagingStatus {
        host_binary_path,
        host_exists,
        sigma_manifest_installed,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct NativeMessagingStatus {
    pub host_binary_path: Option<PathBuf>,
    pub host_exists: bool,
    pub sigma_manifest_installed: bool,
}

/// Tauri command to install native messaging manifests
#[tauri::command]
pub async fn install_native_messaging() -> Result<String, String> {
    install_native_messaging_manifests().map_err(|e| e.to_string())?;
    Ok("Native messaging manifests installed successfully".to_string())
}

/// Tauri command to check native messaging status
#[tauri::command]
pub async fn get_native_messaging_status() -> Result<NativeMessagingStatus, String> {
    check_native_messaging_status().map_err(|e| e.to_string())
}

// Native Messaging Host manifest installation
// Automatically installs the manifest for Sigma browser extension

use anyhow::{Context, Result};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// Extension ID for the Sigma Shield browser extension (loaded from .env at build time)
const EXTENSION_ID: &str = env!("EXTENSION_ID");

/// Native messaging host name
const HOST_NAME: &str = "com.sigma_shield.host";

/// Get the path to the native messaging host binary inside the app bundle
#[cfg(target_os = "macos")]
fn get_host_binary_path() -> Result<PathBuf> {
    // Get the path to the current executable
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    
    // The binary should be in the same directory (Contents/MacOS/)
    let macos_dir = exe_path
        .parent()
        .context("Failed to get MacOS directory")?;
    
    let host_path = macos_dir.join("sigma-shield-host");
    
    if host_path.exists() {
        Ok(host_path)
    } else {
        // Fallback: check if running in development mode
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("sigma-shield-host");
        
        if dev_path.exists() {
            Ok(dev_path)
        } else {
            anyhow::bail!("Native messaging host binary not found")
        }
    }
}

#[cfg(not(target_os = "macos"))]
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

#[cfg(not(target_os = "macos"))]
fn get_sigma_native_hosts_dir() -> Result<PathBuf> {
    anyhow::bail!("Not supported on this platform")
}

/// Generate the manifest JSON content
fn generate_manifest(host_binary_path: &PathBuf) -> String {
    let manifest = json!({
        "name": HOST_NAME,
        "description": "Sigma Shield LLM Native Messaging Host",
        "path": host_binary_path.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": [
            format!("chrome-extension://{}/", EXTENSION_ID)
        ]
    });
    
    serde_json::to_string_pretty(&manifest).unwrap()
}

/// Install the native messaging manifest for a specific browser
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

/// Check if native messaging is properly configured
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

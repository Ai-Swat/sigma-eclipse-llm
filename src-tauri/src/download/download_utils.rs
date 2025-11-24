use crate::types::VersionsConfig;
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::{BufReader, Read};

/// Calculate SHA-256 checksum of a file
pub fn calculate_sha256(file_path: &std::path::Path) -> Result<String, String> {
    let file = File::open(file_path)
        .map_err(|e| format!("Failed to open file for checksum: {}", e))?;
    
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    
    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file for checksum: {}", e))?;
        
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Verify SHA-256 checksum of a file
pub fn verify_sha256(file_path: &std::path::Path, expected_hash: &str) -> Result<(), String> {
    if expected_hash.is_empty() {
        log::warn!("SHA-256 checksum not configured for this file, skipping verification");
        return Ok(());
    }
    
    // Get file size for logging
    let file_size = std::fs::metadata(file_path)
        .map(|m| m.len())
        .unwrap_or(0);
    
    log::info!("Verifying SHA-256 for file: {:?}, size: {} bytes", file_path, file_size);
    
    let calculated_hash = calculate_sha256(file_path)?;
    
    if calculated_hash.to_lowercase() != expected_hash.to_lowercase() {
        return Err(format!(
            "SHA-256 checksum verification failed!\nFile: {:?}\nSize: {} bytes\nExpected: {}\nGot: {}",
            file_path, file_size, expected_hash, calculated_hash
        ));
    }
    
    log::info!("SHA-256 checksum verified successfully: {}", calculated_hash);
    Ok(())
}

/// Get current platform identifier for llama.cpp downloads
pub fn get_platform_id() -> Result<String, String> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok("macos-arm64".to_string());

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Ok("macos-x64".to_string());

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok("linux-x64".to_string());

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok("windows-x64".to_string());

    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    return Err("Unsupported platform".to_string());
}

/// Load configuration from versions.json (includes llama.cpp and models)
pub fn load_config() -> Result<VersionsConfig, String> {
    let config_str = include_str!("../../versions.json");
    serde_json::from_str(config_str).map_err(|e| format!("Failed to parse versions.json: {}", e))
}


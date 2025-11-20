use crate::types::VersionsConfig;

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


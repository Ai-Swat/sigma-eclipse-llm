use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Child;
use std::sync::Mutex;

// Server state management
pub struct ServerState {
    pub process: Mutex<Option<Child>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub is_running: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percentage: Option<f64>,
    pub message: String,
}

// LlamaCpp platform configuration
#[derive(Debug, Deserialize)]
pub struct LlamaCppPlatform {
    pub url: String,
    #[serde(default)]
    pub sha256: String,
}

// LlamaCpp version configuration
#[derive(Debug, Deserialize)]
pub struct LlamaCppConfig {
    pub version: String,
    pub platforms: HashMap<String, LlamaCppPlatform>,
}

// Model configuration from versions.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    pub version: String,
    pub filename: String,
    pub url: String,
    #[serde(default)]
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
pub struct VersionsConfig {
    #[serde(rename = "appVersion")]
    #[allow(dead_code)]
    pub app_version: String,
    #[serde(rename = "llamaCpp")]
    pub llama_cpp: LlamaCppConfig,
    #[serde(default)]
    pub models: HashMap<String, ModelConfig>,
}

// Model information for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub is_downloaded: bool,
    pub path: Option<String>,
}

// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_active_model")]
    pub active_model: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_ctx_size")]
    pub ctx_size: u32,
    #[serde(default = "default_gpu_layers")]
    pub gpu_layers: u32,
}

fn default_active_model() -> String {
    "model".to_string()
}

fn default_port() -> u16 {
    10345
}

fn default_ctx_size() -> u32 {
    8192
}

fn default_gpu_layers() -> u32 {
    0
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            active_model: default_active_model(),
            port: default_port(),
            ctx_size: default_ctx_size(),
            gpu_layers: default_gpu_layers(),
        }
    }
}

// Recommended system settings based on available resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedSettings {
    pub memory_gb: u64,
    pub recommended_model: String,
    pub recommended_ctx_size: u32,
    pub recommended_gpu_layers: u32,
}


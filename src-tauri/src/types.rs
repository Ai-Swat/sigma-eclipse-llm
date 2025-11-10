use serde::{Deserialize, Serialize};
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


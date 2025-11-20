// Download module - coordinates all download operations

mod download_utils;
mod llama_download;
mod model_download;

// Re-export Tauri commands
pub use llama_download::{check_llama_version, download_llama_cpp};
pub use model_download::{
    check_model_downloaded, delete_model, download_model_by_name, list_available_models,
};


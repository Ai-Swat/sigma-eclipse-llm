use std::sync::Mutex;
use tauri::Manager;

// Module declarations
mod download;
mod paths;
mod server;
mod system;
mod types;

// Re-export command functions
use download::{download_llama_cpp, download_model};
use server::{get_server_status, start_server, stop_server};
use system::{clear_all_data, clear_binaries, clear_models, get_app_data_path, get_system_memory_gb};
use types::ServerState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(ServerState {
            process: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            download_llama_cpp,
            download_model,
            start_server,
            stop_server,
            get_server_status,
            get_app_data_path,
            get_system_memory_gb,
            clear_binaries,
            clear_models,
            clear_all_data,
        ])
        .on_window_event(|window, event| {
            // Hide window instead of closing when user clicks close button
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.hide().unwrap_or_else(|e| {
                    eprintln!("Failed to hide window: {}", e);
                });
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                // Handle macOS dock icon click to show window
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen { has_visible_windows, .. } => {
                    if !has_visible_windows {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                _ => {}
            }
        });
}

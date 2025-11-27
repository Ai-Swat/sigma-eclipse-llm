use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::Manager;

// Module declarations
mod download;
pub mod ipc_state;
mod native_messaging;
mod paths;
mod server;
pub mod server_manager;
mod settings;
mod system;
mod types;

// Re-export command functions
use download::{
    check_llama_version, check_model_downloaded, delete_model, download_llama_cpp,
    download_model_by_name, list_available_models,
};
use server::{get_server_status, start_server, stop_server};
use settings::{get_active_model_command, set_active_model_command};
use native_messaging::{get_native_messaging_status, install_native_messaging};
use system::{
    clear_all_data, clear_binaries, clear_models, get_app_data_path, get_logs_path,
    get_recommended_settings, get_system_memory_gb,
};
use types::ServerState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_file_name = format!(
        "sigma-eclipse-{}.log",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // When a second instance is launched, show and focus the first instance's window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    // Write to file in app data directory
                    tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::LogDir {
                            file_name: Some(log_file_name),
                        }
                    ),
                    // Also output to stdout for debugging
                    tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::Stdout
                    ),
                ])
                .level(log::LevelFilter::Info)
                .build()
        )
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(ServerState {
            process: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            check_llama_version,
            download_llama_cpp,
            download_model_by_name,
            list_available_models,
            check_model_downloaded,
            delete_model,
            get_active_model_command,
            set_active_model_command,
            start_server,
            stop_server,
            get_server_status,
            get_app_data_path,
            get_logs_path,
            get_system_memory_gb,
            get_recommended_settings,
            clear_binaries,
            clear_models,
            clear_all_data,
            install_native_messaging,
            get_native_messaging_status,
        ])
        .on_window_event(|window, event| {
            // Hide window instead of closing when user clicks close button
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.hide().unwrap_or_else(|e| {
                    log::error!("Failed to hide window: {}", e);
                });
            }
        })
        .setup(|_app| {
            // Install native messaging manifests on startup
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = native_messaging::install_native_messaging_manifests() {
                    log::warn!("Failed to install native messaging manifests: {}", e);
                }
            }
            
            // Start heartbeat thread to signal that Tauri app is running
            let pid = std::process::id();
            thread::spawn(move || {
                log::info!("Heartbeat thread started for PID: {}", pid);
                loop {
                    if let Err(e) = ipc_state::update_tauri_app_heartbeat(pid) {
                        log::warn!("Failed to update heartbeat: {}", e);
                    }
                    thread::sleep(Duration::from_secs(3));
                }
            });
            
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Register cleanup handler for app termination
    app.run(|app_handle, event| {
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
            // Handle all exit scenarios - stop server before quitting
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                log::info!("App is exiting, stopping server...");
                
                // Clear Tauri app status from IPC state
                if let Err(e) = ipc_state::clear_tauri_app_status() {
                    log::warn!("Failed to clear Tauri app status: {}", e);
                }
                
                // Get server state and stop server if running
                if let Some(state) = app_handle.try_state::<ServerState>() {
                    let mut process_guard = state.process.lock().unwrap();
                    if let Some(mut child) = process_guard.take() {
                        log::info!("Killing server process...");
                        
                        // On Unix, kill the entire process group
                        #[cfg(unix)]
                        {
                            let pid = child.id() as i32;
                            unsafe {
                                libc::kill(-pid, libc::SIGTERM);
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                libc::kill(-pid, libc::SIGKILL);
                            }
                        }
                        
                        let _ = child.kill();
                        let _ = child.wait();
                        log::info!("Server process stopped");
                    }
                }
            }
            _ => {}
        }
    });
}

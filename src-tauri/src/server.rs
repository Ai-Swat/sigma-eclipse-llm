use crate::ipc_state::update_server_status;
use crate::server_manager::{get_status, start_server_process, stop_server_by_pid, ServerConfig};
use crate::settings::get_server_settings;
use crate::types::{ServerState, ServerStatus};
use std::io::{BufRead, BufReader};
use tauri::State;
// Добавьте импорты в начало файла, если их нет
use std::time::{Duration, Instant};
use reqwest::Client; // reqwest уже есть в Cargo.toml

#[tauri::command]
pub async fn start_server(
    state: State<'_, ServerState>,
) -> Result<String, String> {
    // 1. Вся работа с Mutex — СТРОГО внутри этого блока
    let (pid, port, ctx_size, gpu_layers) = {
        let mut process_guard = state.process.lock().unwrap();

        // Проверяем, не запущен ли уже процесс
        if let Some(ref mut child) = *process_guard {
            match child.try_wait() {
                Ok(None) => return Err("Server is already running".to_string()),
                Ok(Some(_)) => *process_guard = None,
                Err(_) => *process_guard = None,
            }
        }

        // Извлекаем настройки и запускаем
        let (port, ctx_size, gpu_layers) = get_server_settings().map_err(|e| e.to_string())?;
        let config = ServerConfig { port, ctx_size, gpu_layers };

        let child = start_server_process(config, true).map_err(|e| e.to_string())?;
        let pid = child.id();
        
        // Сохраняем ребенка в стейт
        *process_guard = Some(child);
        
        // Возвращаем кортеж данных наружу из блока
        (pid, port, ctx_size, gpu_layers)
        
        // Тут process_guard выходит из области видимости и САМ делает unlock()
    };

    // 2. Теперь мы ВНЕ блока. Мьютекс свободен. Можно юзать .await
    log::info!("Waiting for LLM server to be ready on port {}...", port);
    
    match wait_for_server_ready(port, 60).await {
        Ok(_) => {
            log::info!("LLM Server is ready!");
            Ok(format!(
                "Server started on port {} (PID: {}, ctx: {}, gpu layers: {})",
                port, pid, ctx_size, gpu_layers
            ))
        }
        Err(e) => {
            log::error!("Server failed to start check: {}", e);
            // Тут вызываем стоп, так как сервер завис или не отвечает
            let _ = stop_server(state.clone()).await; 
            Err(format!("Failed to start server: {}", e))
        }
    }
}


#[tauri::command]
pub async fn stop_server(state: State<'_, ServerState>) -> Result<String, String> {
    let mut process_guard = state.process.lock().unwrap();

    if let Some(mut child) = process_guard.take() {
        let pid = child.id();
        
        // Use shared server manager to stop
        stop_server_by_pid(pid).map_err(|e| e.to_string())?;
        
        // Also clean up local Child handle
        let _ = child.kill();
        let _ = child.wait();
        
        Ok("Server stopped".to_string())
    } else {
        // Check if server is running elsewhere (e.g., via Native Host)
        if let Ok((is_running, Some(pid))) = get_status() {
            if is_running {
                stop_server_by_pid(pid).map_err(|e| e.to_string())?;
                return Ok(format!("Server stopped (PID: {})", pid));
            }
        }
        
        Err("LLM is not running".to_string())
    }
}

#[tauri::command]
pub async fn get_server_status(state: State<'_, ServerState>) -> Result<ServerStatus, String> {
    let mut process_guard = state.process.lock().unwrap();

    // First check local process
    if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(None) => {
                return Ok(ServerStatus {
                    is_running: true,
                    message: "LLM is running".to_string(),
                });
            }
            Ok(Some(status)) => {
                *process_guard = None;
                // Update IPC state
                let _ = update_server_status(false, None);
                return Ok(ServerStatus {
                    is_running: false,
                    message: format!("LLM exited with status: {}", status),
                });
            }
            Err(e) => {
                *process_guard = None;
                // Update IPC state
                let _ = update_server_status(false, None);
                return Ok(ServerStatus {
                    is_running: false,
                    message: format!("Failed to check LLM status: {}", e),
                });
            }
        }
    }

    // Check shared IPC state (may be running via Native Host)
    match get_status() {
        Ok((is_running, pid)) => Ok(ServerStatus {
            is_running,
            message: if is_running {
                format!("LLM is running (PID: {})", pid.unwrap_or(0))
            } else {
                "LLM is not running".to_string()
            },
        }),
        Err(e) => Ok(ServerStatus {
            is_running: false,
            message: format!("Failed to check status: {}", e),
        }),
    }
}


// Новая вспомогательная функция
async fn wait_for_server_ready(port: u16, timeout_secs: u64) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .map_err(|e| e.to_string())?;

    let start_time = Instant::now();
    
    loop {
        if start_time.elapsed().as_secs() > timeout_secs {
            return Err("Timeout waiting for LLM server to become ready".to_string());
        }

        // Пытаемся стукнуться в /health (стандартный эндпоинт llama.cpp)
        match client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    return Ok(()); // Сервер ответил 200 OK
                }
            }
            Err(_) => {
                // Сервер еще не поднялся, ждем и пробуем снова
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

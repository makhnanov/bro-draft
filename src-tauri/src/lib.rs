use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use std::time::Duration;
use sysinfo::Components;
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Default)]
struct AppState {
    devtools_open: bool,
}

fn get_config_path() -> Option<PathBuf> {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "bro", "bro") {
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir).ok()?;
        Some(config_dir.join("state.json"))
    } else {
        None
    }
}

fn load_state() -> AppState {
    if let Some(config_path) = get_config_path() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(state) = serde_json::from_str(&content) {
                return state;
            }
        }
    }
    AppState::default()
}

fn save_state(state: &AppState) {
    if let Some(config_path) = get_config_path() {
        if let Ok(content) = serde_json::to_string_pretty(state) {
            let _ = fs::write(config_path, content);
        }
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_cpu_temperature() -> Result<String, String> {
    let mut components = Components::new_with_refreshed_list();
    components.refresh();

    if components.is_empty() {
        return Ok("Датчики температуры не найдены на этой системе".to_string());
    }

    let mut result = String::new();
    result.push_str("Температура процессора:\n\n");

    let mut found_cpu = false;

    for component in &components {
        let label = component.label();
        let temp = component.temperature();

        // Фильтруем только компоненты связанные с CPU
        if label.to_lowercase().contains("cpu")
            || label.to_lowercase().contains("core")
            || label.to_lowercase().contains("processor") {
            result.push_str(&format!("{}: {:.1}°C\n", label, temp));
            found_cpu = true;
        }
    }

    if !found_cpu {
        // Если не нашли CPU компоненты, покажем все
        result.push_str("Не найдено специфичных датчиков CPU.\n");
        result.push_str("Доступные датчики:\n\n");
        for component in &components {
            result.push_str(&format!("{}: {:.1}°C\n", component.label(), component.temperature()));
        }
    }

    Ok(result)
}

#[tauri::command]
async fn get_network_speed() -> Result<String, String> {
    use tokio::process::Command;

    println!("=== Starting network speed test ===");

    // Пробуем сначала официальный speedtest от Ookla
    println!("Trying official speedtest from Ookla...");
    let speedtest_result = Command::new("speedtest")
        .arg("--simple")
        .output()
        .await;

    match speedtest_result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            println!("speedtest SUCCESS!");
            println!("Status: {:?}", output.status);
            println!("STDOUT length: {}", stdout.len());
            println!("STDERR length: {}", stderr.len());
            println!("STDOUT:\n{}", stdout);
            println!("STDERR:\n{}", stderr);

            // Возвращаем stdout, а если он пустой - stderr (некоторые программы выводят в stderr)
            let result = if !stdout.is_empty() {
                stdout
            } else if !stderr.is_empty() {
                stderr
            } else {
                "Тест выполнен, но результаты отсутствуют".to_string()
            };

            println!("Returning result length: {}", result.len());
            return Ok(result);
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("speedtest FAILED, trying speedtest-cli...");
            println!("Status: {:?}", output.status);
            println!("STDOUT: {}", stdout);
            println!("STDERR: {}", stderr);
            eprintln!("speedtest failed, trying speedtest-cli...");
            eprintln!("STDERR: {}", stderr);
        }
        Err(e) => {
            println!("speedtest NOT FOUND: {}", e);
            println!("Trying speedtest-cli instead...");
            eprintln!("speedtest not found ({}), trying speedtest-cli...", e);
        }
    }

    // Если speedtest не сработал, пробуем speedtest-cli
    println!("Trying speedtest-cli...");
    let output = Command::new("speedtest-cli")
        .arg("--simple")
        .output()
        .await
        .map_err(|e| {
            let err_msg = format!("Ошибка запуска speedtest-cli: {}", e);
            println!("ERROR: {}", err_msg);
            eprintln!("ERROR: {}", err_msg);
            err_msg
        })?;

    println!("speedtest-cli finished with status: {:?}", output.status);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        println!("speedtest-cli FAILED!");
        println!("Status: {:?}", output.status);
        println!("STDOUT: {}", stdout);
        println!("STDERR: {}", stderr);

        eprintln!("speedtest-cli failed:");
        eprintln!("Status: {:?}", output.status);
        eprintln!("STDOUT: {}", stdout);
        eprintln!("STDERR: {}", stderr);

        return Err(format!(
            "speedtest-cli завершился с ошибкой\nStatus: {:?}\nSTDOUT: {}\nSTDERR: {}\n\nПопробуйте установить официальный speedtest: https://www.speedtest.net/apps/cli",
            output.status, stdout, stderr
        ));
    }

    let result = String::from_utf8_lossy(&output.stdout).to_string();
    println!("speedtest-cli SUCCESS!");
    println!("OUTPUT:\n{}", result);
    println!("=== Network speed test completed ===");

    Ok(result)
}

// Wait for dev server to be ready
fn wait_for_dev_server(url: &str, max_attempts: u32) -> bool {
    for attempt in 1..=max_attempts {
        match reqwest::blocking::get(url) {
            Ok(_) => {
                println!("Dev server is ready after {} attempt(s)", attempt);
                return true;
            }
            Err(_) => {
                if attempt < max_attempts {
                    println!("Waiting for dev server... (attempt {}/{})", attempt, max_attempts);
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }
    println!("Dev server didn't respond after {} attempts", max_attempts);
    false
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Wait for dev server in development mode
    #[cfg(debug_assertions)]
    {
        println!("Development mode: waiting for dev server...");
        wait_for_dev_server("http://localhost:1420", 20);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            app.global_shortcut().on_shortcut("F12", |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    if let Some(window) = app.get_webview_window("main") {
                        let is_open = window.is_devtools_open();
                        if is_open {
                            let _ = window.close_devtools();
                        } else {
                            let _ = window.open_devtools();
                        }
                        // Сохраняем новое состояние
                        save_state(&AppState {
                            devtools_open: !is_open,
                        });
                    }
                }
            })?;

            app.global_shortcut().on_shortcut("F5", |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    if let Some(webview) = app.get_webview_window("main") {
                        let _ = webview.eval("window.location.reload()");
                    }
                }
            })?;

            app.global_shortcut().on_shortcut("F11", |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    if let Some(window) = app.get_webview_window("main") {
                        match window.is_fullscreen() {
                            Ok(is_fullscreen) => {
                                let _ = window.set_fullscreen(!is_fullscreen);
                            }
                            Err(_) => {
                                // В случае ошибки просто выключаем fullscreen
                                let _ = window.set_fullscreen(false);
                            }
                        }
                    }
                }
            })?;

            // Восстанавливаем состояние DevTools
            let saved_state = load_state();
            println!("Loaded state: devtools_open = {}", saved_state.devtools_open);

            if saved_state.devtools_open {
                if let Some(window) = app.get_webview_window("main") {
                    println!("Opening DevTools on startup...");
                    let _ = window.open_devtools();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, get_cpu_temperature, get_network_speed])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

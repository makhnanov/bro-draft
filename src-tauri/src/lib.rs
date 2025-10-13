use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use std::time::Duration;
use sysinfo::{System, Components};

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
                        if window.is_devtools_open() {
                            let _ = window.close_devtools();
                        } else {
                            let _ = window.open_devtools();
                        }
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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, get_cpu_temperature])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

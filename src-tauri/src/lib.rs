use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use std::time::Duration;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

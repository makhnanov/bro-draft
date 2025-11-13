use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use std::time::Duration;
use sysinfo::Components;
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use screenshots::Screen;

#[derive(Serialize, Deserialize)]
struct AppState {
    devtools_open: bool,
    window_x: Option<i32>,
    window_y: Option<i32>,
    window_width: Option<u32>,
    window_height: Option<u32>,
    translation_hotkey: Option<String>,
    last_route: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            devtools_open: false,
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
            translation_hotkey: None,
            last_route: Some("/".to_string()),
        }
    }
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

// Команда для захвата полного экрана
#[tauri::command]
async fn capture_full_screenshot() -> Result<String, String> {
    use png::Encoder;
    use png::ColorType;
    use std::io::BufWriter;

    println!("Capturing full screenshot...");

    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    if screens.is_empty() {
        return Err("No screens found".to_string());
    }

    // Используем первый экран
    let screen = &screens[0];
    let captured_image = screen.capture().map_err(|e| format!("Failed to capture screen: {}", e))?;

    // Кодируем изображение в PNG
    let width = captured_image.width();
    let height = captured_image.height();

    // Получаем RAW данные из изображения
    let rgba_data: Vec<u8> = captured_image.rgba().to_vec();

    // Кодируем в PNG
    let mut png_data = Vec::new();
    {
        let w = BufWriter::new(&mut png_data);
        let mut encoder = Encoder::new(w, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()
            .map_err(|e| format!("Failed to write PNG header: {}", e))?;

        writer.write_image_data(&rgba_data)
            .map_err(|e| format!("Failed to write PNG data: {}", e))?;
    }

    // Конвертируем в base64
    let base64_image = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_data);

    println!("Screenshot captured successfully");
    Ok(base64_image)
}

use std::sync::Mutex;
use std::collections::HashMap;

// Глобальное состояние для хранения скриншотов (по одному на монитор)
struct ScreenshotState {
    data: Mutex<HashMap<usize, String>>, // monitor_index -> base64 screenshot
}

// Команда для получения сохранённого скриншота по индексу монитора
#[tauri::command]
fn get_stored_screenshot(monitor_index: usize, state: tauri::State<ScreenshotState>) -> Result<String, String> {
    println!("get_stored_screenshot called for monitor {}", monitor_index);
    let screenshots = state.data.lock().unwrap();

    match screenshots.get(&monitor_index) {
        Some(data) => {
            println!("Screenshot found for monitor {}, length: {}", monitor_index, data.len());
            Ok(data.clone())
        }
        None => {
            println!("No screenshot for monitor {}", monitor_index);
            Err(format!("No screenshot available for monitor {}", monitor_index))
        }
    }
}

// Команда для открытия окна выбора области поверх всей ОС
#[tauri::command]
async fn open_area_selector(app_handle: tauri::AppHandle, state: tauri::State<'_, ScreenshotState>) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;
    use tauri::WebviewUrl;
    use png::Encoder;
    use png::ColorType;
    use std::io::BufWriter;

    println!("Capturing screenshot for area selection...");

    // Закрываем существующие окна area-selector если они есть
    let mut monitor_index = 0;
    loop {
        let window_label = format!("area-selector-{}", monitor_index);
        if let Some(existing_window) = app_handle.get_webview_window(&window_label) {
            println!("Closing existing area selector window: {}", window_label);
            let _ = existing_window.close();
        } else {
            break;
        }
        monitor_index += 1;
    }

    // Даём время на закрытие окон
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Получаем все экраны
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    if screens.is_empty() {
        return Err("No screens found".to_string());
    }

    println!("Found {} screen(s)", screens.len());

    // Захватываем скриншот каждого монитора отдельно
    let mut screenshots_map = HashMap::new();

    for (index, screen) in screens.iter().enumerate() {
        let display = screen.display_info;
        println!("Capturing screen {}: x={}, y={}, width={}, height={}",
            index, display.x, display.y, display.width, display.height);

        // Захватываем скриншот этого монитора
        let captured = screen.capture().map_err(|e| format!("Failed to capture screen {}: {}", index, e))?;

        let width = captured.width();
        let height = captured.height();
        let rgba_data = captured.rgba();

        // Кодируем в PNG
        let mut png_data = Vec::new();
        {
            let w = BufWriter::new(&mut png_data);
            let mut encoder = Encoder::new(w, width, height);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder.write_header()
                .map_err(|e| format!("Failed to write PNG header for screen {}: {}", index, e))?;

            writer.write_image_data(rgba_data)
                .map_err(|e| format!("Failed to write PNG data for screen {}: {}", index, e))?;
        }

        // Конвертируем в base64
        let base64_screenshot = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_data);
        println!("Screenshot {} size: {} bytes ({}x{})", index, base64_screenshot.len(), width, height);

        screenshots_map.insert(index, base64_screenshot);
    }

    // Сохраняем все скриншоты в state
    {
        let mut screenshots = state.data.lock().unwrap();
        *screenshots = screenshots_map;
    }

    println!("Screenshots saved to state, creating windows for each monitor...");

    // Создаём окно для каждого монитора
    for (index, screen) in screens.iter().enumerate() {
        let display = screen.display_info;
        let window_label = format!("area-selector-{}", index);

        println!("Creating window {} for monitor {} at ({}, {})", window_label, index, display.x, display.y);

        let webview_window = WebviewWindowBuilder::new(
            &app_handle,
            &window_label,
            WebviewUrl::App(format!("/index.html#/area-selector?monitor={}", index).into())
        )
        .title(&format!("Select Area - Monitor {}", index))
        .position(display.x as f64, display.y as f64)
        .inner_size(display.width as f64, display.height as f64)
        .decorations(false)
        .transparent(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(true)
        .resizable(false)
        .build()
        .map_err(|e| format!("Failed to create area selector window {}: {}", index, e))?;

        println!("Window {} created for monitor {}", window_label, index);

        // Переводим окно в полноэкранный режим после создания
        webview_window.set_fullscreen(true)
            .map_err(|e| format!("Failed to set fullscreen for window {}: {}", index, e))?;

        let _ = webview_window.set_always_on_top(true);
        let _ = webview_window.set_focus();

        println!("Window {} shown in fullscreen for monitor {}", window_label, index);
    }

    // Регистрируем глобальную горячую клавишу ESC для закрытия всех окон (если ещё не зарегистрирована)
    let app_clone = app_handle.clone();
    let register_result = app_handle.global_shortcut().on_shortcut("Escape", move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            println!("ESC shortcut pressed! Closing all area-selector windows");
            let mut monitor_idx = 0;
            loop {
                let window_label = format!("area-selector-{}", monitor_idx);
                if let Some(win) = app_clone.get_webview_window(&window_label) {
                    println!("Closing window: {}", window_label);
                    let _ = win.close();
                    monitor_idx += 1;
                } else {
                    break;
                }
            }
        }
    });

    match register_result {
        Ok(_) => println!("ESC shortcut registered for all area-selector windows"),
        Err(e) => {
            println!("ESC shortcut already registered or failed: {}", e);
            // Не возвращаем ошибку, так как это нормально если уже зарегистрирована
        }
    }

    Ok(())
}

// Команда для закрытия всех окон area-selector
#[tauri::command]
async fn close_all_area_selectors(app_handle: tauri::AppHandle) -> Result<(), String> {
    println!("Closing all area-selector windows...");
    let mut monitor_index = 0;
    loop {
        let window_label = format!("area-selector-{}", monitor_index);
        if let Some(window) = app_handle.get_webview_window(&window_label) {
            println!("Closing window: {}", window_label);
            let _ = window.close();
            monitor_index += 1;
        } else {
            break;
        }
    }
    println!("All area-selector windows closed");
    Ok(())
}

// Команда для обработки выбора области и отправки события в главное окно
#[tauri::command]
async fn handle_area_selection(app_handle: tauri::AppHandle, x: u32, y: u32, width: u32, height: u32) -> Result<(), String> {
    use tauri::Emitter;

    println!("Handling area selection: x={}, y={}, width={}, height={}", x, y, width, height);

    // Закрываем все окна area-selector
    close_all_area_selectors(app_handle.clone()).await?;

    // Отправляем событие в главное окно
    if let Some(main_window) = app_handle.get_webview_window("main") {
        println!("Emitting area-selected event to main window...");
        let payload = serde_json::json!({
            "x": x,
            "y": y,
            "width": width,
            "height": height
        });
        main_window.emit("area-selected", payload)
            .map_err(|e| format!("Failed to emit event: {}", e))?;
        println!("Event emitted successfully");
    } else {
        return Err("Main window not found".to_string());
    }

    Ok(())
}

// Команда для обработки выбранной области
#[tauri::command]
async fn capture_area_screenshot(x: u32, y: u32, width: u32, height: u32) -> Result<String, String> {
    use png::Encoder;
    use png::ColorType;
    use std::io::BufWriter;

    println!("Capturing area screenshot: x={}, y={}, width={}, height={}", x, y, width, height);

    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    if screens.is_empty() {
        return Err("No screens found".to_string());
    }

    // Используем первый экран
    let screen = &screens[0];
    let full_image = screen.capture().map_err(|e| format!("Failed to capture screen: {}", e))?;

    // Получаем RAW данные из изображения
    let full_rgba_data: Vec<u8> = full_image.rgba().to_vec();
    let full_width = full_image.width();
    let full_height = full_image.height();

    // Проверяем границы
    if x + width > full_width || y + height > full_height {
        return Err("Selection area out of bounds".to_string());
    }

    // Обрезаем изображение
    let mut cropped_data = Vec::with_capacity((width * height * 4) as usize);

    for row in y..(y + height) {
        let start_idx = ((row * full_width + x) * 4) as usize;
        let end_idx = start_idx + (width * 4) as usize;
        cropped_data.extend_from_slice(&full_rgba_data[start_idx..end_idx]);
    }

    // Кодируем обрезанное изображение в PNG
    let mut png_data = Vec::new();
    {
        let w = BufWriter::new(&mut png_data);
        let mut encoder = Encoder::new(w, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()
            .map_err(|e| format!("Failed to write PNG header: {}", e))?;

        writer.write_image_data(&cropped_data)
            .map_err(|e| format!("Failed to write PNG data: {}", e))?;
    }

    // Конвертируем в base64
    let base64_image = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_data);

    println!("Area screenshot captured successfully");
    Ok(base64_image)
}

// Команда для сохранения горячей клавиши
#[tauri::command]
fn save_translation_hotkey(hotkey: String) -> Result<(), String> {
    let mut state = load_state();
    state.translation_hotkey = Some(hotkey.clone());
    save_state(&state);
    println!("Translation hotkey saved: {}", hotkey);
    Ok(())
}

// Команда для получения сохраненной горячей клавиши
#[tauri::command]
fn get_translation_hotkey() -> Result<Option<String>, String> {
    let state = load_state();
    Ok(state.translation_hotkey)
}

// Команда для сохранения последнего маршрута
#[tauri::command]
fn save_last_route(route: String) -> Result<(), String> {
    let mut state = load_state();
    state.last_route = Some(route.clone());
    save_state(&state);
    println!("Last route saved: {}", route);
    Ok(())
}

// Команда для получения последнего маршрута
#[tauri::command]
fn get_last_route() -> Result<Option<String>, String> {
    let state = load_state();
    println!("Last route loaded: {:?}", state.last_route);
    Ok(state.last_route)
}

// Команда для отправки изображения в ChatGPT
#[tauri::command]
async fn send_to_chatgpt(api_key: String, image_base64: String, prompt: String) -> Result<String, String> {
    use reqwest;

    println!("Sending to ChatGPT...");

    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", image_base64)
                        }
                    }
                ]
            }
        ],
        "max_tokens": 1000
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("ChatGPT API error: {}", error_text));
    }

    let response_json: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?
        .to_string();

    println!("ChatGPT response received");
    Ok(content)
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
        .manage(ScreenshotState {
            data: Mutex::new(HashMap::new()),
        })
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
                        let mut state = load_state();
                        state.devtools_open = !is_open;
                        save_state(&state);
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

            // Регистрируем горячую клавишу для переводов, если она сохранена
            let saved_state = load_state();
            if let Some(translation_hotkey) = saved_state.translation_hotkey.clone() {
                let hotkey_str = translation_hotkey.clone();
                println!("Registering translation hotkey: {}", hotkey_str);

                match app.global_shortcut().on_shortcut(hotkey_str.as_str(), move |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        println!("Translation hotkey pressed!");
                        if let Some(window) = app.get_webview_window("main") {
                            // Отправляем событие в фронтенд для захвата скриншота
                            let _ = window.eval("window.dispatchEvent(new CustomEvent('translation-hotkey-pressed'))");
                        }
                    }
                }) {
                    Ok(_) => println!("Translation hotkey registered successfully"),
                    Err(e) => eprintln!("Failed to register translation hotkey: {}", e),
                }
            }

            app.global_shortcut().on_shortcut("CommandOrControl+Shift+C", |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    if let Some(window) = app.get_webview_window("main") {
                        println!("Opening DevTools with inspector mode...");

                        // Проверяем, открыты ли DevTools
                        let was_closed = !window.is_devtools_open();

                        // Открываем DevTools если они закрыты
                        if was_closed {
                            let _ = window.open_devtools();

                            // Сохраняем состояние
                            let mut state = load_state();
                            state.devtools_open = true;
                            save_state(&state);
                        }

                        // Активируем инспектор элементов
                        // После открытия DevTools нужно небольшое ожидание
                        let delay = if was_closed { 700 } else { 100 };
                        let window_clone = window.clone();

                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(delay));

                            // Попытка активировать инспектор через JavaScript
                            let _ = window_clone.eval(
                                r#"
                                (function() {
                                    console.log('Attempting to activate element inspector...');

                                    // Способ 1: Эмуляция нажатия Ctrl+Shift+C
                                    function triggerInspector() {
                                        const events = ['keydown', 'keypress', 'keyup'];

                                        events.forEach(eventType => {
                                            const event = new KeyboardEvent(eventType, {
                                                key: 'C',
                                                code: 'KeyC',
                                                keyCode: 67,
                                                which: 67,
                                                charCode: eventType === 'keypress' ? 67 : 0,
                                                ctrlKey: true,
                                                shiftKey: true,
                                                altKey: false,
                                                metaKey: false,
                                                bubbles: true,
                                                cancelable: true,
                                                composed: true
                                            });

                                            // Отправляем события на разные цели
                                            document.dispatchEvent(event);
                                            window.dispatchEvent(event);
                                            document.body && document.body.dispatchEvent(event);
                                        });
                                    }

                                    // Запускаем с задержками для надежности
                                    triggerInspector();
                                    setTimeout(triggerInspector, 50);
                                    setTimeout(triggerInspector, 150);

                                    console.log('Element inspector activation triggered');
                                })();
                                "#
                            );

                            println!("Inspector element picker requested");
                        });
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

            // Восстанавливаем положение и размер окна
            if let Some(window) = app.get_webview_window("main") {
                if let (Some(x), Some(y)) = (saved_state.window_x, saved_state.window_y) {
                    println!("Restoring window position: x={}, y={}", x, y);
                    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
                }
                if let (Some(width), Some(height)) = (saved_state.window_width, saved_state.window_height) {
                    println!("Restoring window size: {}x{}", width, height);
                    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }));
                }

                // Слушаем события изменения положения и размера окна
                window.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::Moved(position) => {
                            let mut state = load_state();
                            state.window_x = Some(position.x);
                            state.window_y = Some(position.y);
                            save_state(&state);
                        }
                        tauri::WindowEvent::Resized(size) => {
                            let mut state = load_state();
                            state.window_width = Some(size.width);
                            state.window_height = Some(size.height);
                            save_state(&state);
                        }
                        _ => {}
                    }
                });

                // Периодически проверяем состояние DevTools и сохраняем
                let window_clone = window.clone();
                std::thread::spawn(move || {
                    let mut last_state = window_clone.is_devtools_open();
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        let current_state = window_clone.is_devtools_open();
                        if current_state != last_state {
                            let mut state = load_state();
                            state.devtools_open = current_state;
                            save_state(&state);
                            last_state = current_state;
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_cpu_temperature,
            get_network_speed,
            capture_full_screenshot,
            open_area_selector,
            get_stored_screenshot,
            capture_area_screenshot,
            close_all_area_selectors,
            handle_area_selection,
            save_translation_hotkey,
            get_translation_hotkey,
            save_last_route,
            get_last_route,
            send_to_chatgpt
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

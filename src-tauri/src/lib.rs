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
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    auto_open_links: Option<bool>,
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
            openai_api_key: None,
            anthropic_api_key: None,
            auto_open_links: None,
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

// Глобальное состояние для данных popup окна перевода
struct PopupState {
    image_data: Mutex<Option<String>>, // base64 screenshot для popup
    screen_x: Mutex<i32>, // X координата скриншота на экране
    screen_y: Mutex<i32>, // Y координата скриншота на экране
}

// Структура для записи кликов
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ClickPoint {
    x: i32,
    y: i32,
    monitor: usize,
    button: String, // "left" или "right"
}

// Глобальное состояние для записи кликов
struct ClickRecordingState {
    is_recording: Mutex<bool>,
    clicks: std::sync::Arc<Mutex<Vec<ClickPoint>>>,
}

// Глобальная переменная для остановки записи
static STOP_RECORDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
    for monitor_index in 0..10 {
        let window_label = format!("area-selector-{}", monitor_index);
        if let Some(existing_window) = app_handle.get_webview_window(&window_label) {
            println!("Destroying existing area selector window: {}", window_label);
            let _ = existing_window.destroy();
        }
    }

    // Небольшая пауза для завершения операций
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

    Ok(())
}

// Команда для закрытия всех окон area-selector
#[tauri::command]
async fn close_all_area_selectors(app_handle: tauri::AppHandle) -> Result<(), String> {
    println!("Closing all area-selector windows...");
    let mut monitor_index = 0;
    let mut closed_count = 0;
    loop {
        let window_label = format!("area-selector-{}", monitor_index);
        if let Some(window) = app_handle.get_webview_window(&window_label) {
            println!("Closing window: {}", window_label);
            match window.close() {
                Ok(_) => closed_count += 1,
                Err(e) => println!("Warning: Failed to close window {}: {}", window_label, e),
            }
            monitor_index += 1;
        } else {
            break;
        }
    }
    if closed_count > 0 {
        println!("Closed {} area-selector window(s)", closed_count);
        // Даём время на закрытие окон
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    } else {
        println!("No area-selector windows to close");
    }
    Ok(())
}

// Команда для обработки выбора области и отправки события в главное окно
#[tauri::command]
async fn handle_area_selection(app_handle: tauri::AppHandle, x: u32, y: u32, width: u32, height: u32, monitor_index: usize) -> Result<(), String> {
    use tauri::Emitter;

    println!("Handling area selection: x={}, y={}, width={}, height={}, monitor={}", x, y, width, height, monitor_index);

    // Получаем информацию о мониторе для абсолютных координат
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let (monitor_x, monitor_y) = if let Some(screen) = screens.get(monitor_index) {
        (screen.display_info.x, screen.display_info.y)
    } else {
        (0, 0)
    };

    // Закрываем все окна area-selector
    close_all_area_selectors(app_handle.clone()).await?;

    // Отправляем событие в главное окно
    if let Some(main_window) = app_handle.get_webview_window("main") {
        println!("Emitting area-selected event to main window...");
        let payload = serde_json::json!({
            "x": x,
            "y": y,
            "width": width,
            "height": height,
            "monitorIndex": monitor_index,
            "monitorX": monitor_x,
            "monitorY": monitor_y
        });
        main_window.emit("area-selected", payload)
            .map_err(|e| format!("Failed to emit event: {}", e))?;
        println!("Event emitted successfully");
    } else {
        return Err("Main window not found".to_string());
    }

    Ok(())
}

// Команда для открытия popup окна с переводом в позиции выбранной области
#[tauri::command]
async fn open_translation_popup(app_handle: tauri::AppHandle, x: i32, y: i32, width: u32, height: u32, image_base64: String, popup_state: tauri::State<'_, PopupState>) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;
    use tauri::WebviewUrl;

    println!("Opening translation popup at x={}, y={}, size={}x{}", x, y, width, height);

    // Сохраняем данные скриншота и координаты в state
    {
        let mut data = popup_state.image_data.lock().unwrap();
        *data = Some(image_base64);
    }
    {
        let mut sx = popup_state.screen_x.lock().unwrap();
        *sx = x;
    }
    {
        let mut sy = popup_state.screen_y.lock().unwrap();
        *sy = y;
    }

    // Закрываем существующее popup окно если есть
    if let Some(existing) = app_handle.get_webview_window("translation-popup") {
        let _ = existing.close();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Рассчитываем размер окна на основе размера изображения
    let header_height = 54i32; // Высота кастомного заголовка
    let content_padding = 15i32; // Padding в .popup-content
    let buttons_height = 60i32; // Высота кнопок (1 ряд)

    let popup_width = width as i32 + content_padding * 2;
    let popup_height = height as i32 + header_height + buttons_height + content_padding * 2;

    // Позиционируем окно так, чтобы изображение было на месте выбора
    let popup_x = x - content_padding;
    let popup_y = y - header_height - content_padding - 9;

    // Создаём окно за пределами экрана чтобы избежать анимации compositor
    let webview_window = WebviewWindowBuilder::new(
        &app_handle,
        "translation-popup",
        WebviewUrl::App("/index.html#/translation-popup".into())
    )
    .title("Перевод")
    .position(-10000.0, -10000.0)  // За пределами экрана
    .inner_size(popup_width as f64, popup_height as f64)
    .decorations(false)
    .transparent(true)
    .always_on_top(false)
    .skip_taskbar(false)
    .visible(true)
    .resizable(true)
    .build()
    .map_err(|e| format!("Failed to create translation popup: {}", e))?;

    // Перемещаем в нужную позицию
    let _ = webview_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
        x: popup_x,
        y: popup_y
    }));
    let _ = webview_window.set_focus();

    println!("Translation popup opened successfully");
    Ok(())
}

// Команда для получения данных скриншота в popup
#[tauri::command]
fn get_popup_screenshot(popup_state: tauri::State<'_, PopupState>) -> Option<String> {
    let data = popup_state.image_data.lock().unwrap();
    data.clone()
}

// Команда для получения координат скриншота на экране
#[tauri::command]
fn get_popup_screen_position(popup_state: tauri::State<'_, PopupState>) -> (i32, i32) {
    let x = *popup_state.screen_x.lock().unwrap();
    let y = *popup_state.screen_y.lock().unwrap();
    (x, y)
}

// Команда для закрытия popup окна
#[tauri::command]
async fn close_translation_popup(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("translation-popup") {
        window.close().map_err(|e| format!("Failed to close popup: {}", e))?;
    }
    Ok(())
}

// Команда для закрытия popup и выполнения клика с визуальной отметкой
#[tauri::command]
async fn solve_and_click(app_handle: tauri::AppHandle, x: i32, y: i32, answer: String) -> Result<(), String> {
    use std::process::Command;

    println!("Solve and click: ({}, {}) - {}", x, y, answer);

    // Закрываем popup окно
    if let Some(window) = app_handle.get_webview_window("translation-popup") {
        let _ = window.close();
    }

    // Выполняем остальное в отдельном потоке
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        // Ждём 1 секунду
        std::thread::sleep(Duration::from_millis(1000));

        // Получаем текущую позицию курсора
        let mut current_x = 0i32;
        let mut current_y = 0i32;

        let output = Command::new("xdotool")
            .args(&["getmouselocation", "--shell"])
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("X=") {
                    current_x = line[2..].parse().unwrap_or(0);
                } else if line.starts_with("Y=") {
                    current_y = line[2..].parse().unwrap_or(0);
                }
            }
        }

        // Плавное перемещение курсора
        let steps = 50;
        let dx = (x - current_x) as f64 / steps as f64;
        let dy = (y - current_y) as f64 / steps as f64;

        println!("Moving cursor from ({}, {}) to ({}, {})", current_x, current_y, x, y);

        for step in 1..=steps {
            let intermediate_x = current_x + (dx * step as f64) as i32;
            let intermediate_y = current_y + (dy * step as f64) as i32;

            let _ = Command::new("xdotool")
                .args(&["mousemove", &intermediate_x.to_string(), &intermediate_y.to_string()])
                .status();

            std::thread::sleep(Duration::from_millis(10));
        }

        // Финальная позиция
        let _ = Command::new("xdotool")
            .args(&["mousemove", &x.to_string(), &y.to_string()])
            .status();

        std::thread::sleep(Duration::from_millis(100));

        // Кликаем
        let _ = Command::new("xdotool")
            .arg("click")
            .arg("1")
            .status();

        println!("Click performed at ({}, {})", x, y);

        Ok(())
    }).await.map_err(|e| format!("Task failed: {}", e))?
}

// Команда для перемещения popup окна
#[tauri::command]
async fn move_translation_popup(app_handle: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("translation-popup") {
        window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
            .map_err(|e| format!("Failed to move popup: {}", e))?;
    }
    Ok(())
}

// Команда для получения позиции popup окна
#[tauri::command]
async fn get_translation_popup_position(app_handle: tauri::AppHandle) -> Result<(i32, i32), String> {
    if let Some(window) = app_handle.get_webview_window("translation-popup") {
        let pos = window.outer_position()
            .map_err(|e| format!("Failed to get popup position: {}", e))?;
        Ok((pos.x, pos.y))
    } else {
        Err("Popup window not found".to_string())
    }
}

// Структура для размера окна
#[derive(Serialize)]
struct WindowSize {
    width: u32,
    height: u32,
}

// Команда для получения размера popup окна
#[tauri::command]
async fn get_window_size(app_handle: tauri::AppHandle) -> Result<WindowSize, String> {
    if let Some(window) = app_handle.get_webview_window("translation-popup") {
        let size = window.inner_size()
            .map_err(|e| format!("Failed to get window size: {}", e))?;
        Ok(WindowSize {
            width: size.width,
            height: size.height,
        })
    } else {
        Err("Popup window not found".to_string())
    }
}

// Команда для установки размера popup окна
#[tauri::command]
async fn set_window_size(app_handle: tauri::AppHandle, width: u32, height: u32) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("translation-popup") {
        window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
            .map_err(|e| format!("Failed to set window size: {}", e))?;
        Ok(())
    } else {
        Err("Popup window not found".to_string())
    }
}

// Команда для обработки выбранной области - вырезает из сохранённого скриншота
#[tauri::command]
async fn capture_area_screenshot(x: u32, y: u32, width: u32, height: u32, monitor_index: usize, state: tauri::State<'_, ScreenshotState>) -> Result<String, String> {
    use png::Encoder;
    use png::ColorType;
    use std::io::BufWriter;

    println!("Cutting area from saved screenshot: x={}, y={}, width={}, height={}, monitor={}", x, y, width, height, monitor_index);

    // Получаем сохранённый скриншот для указанного монитора
    let screenshots = state.data.lock().unwrap();
    let base64_screenshot = screenshots.get(&monitor_index)
        .ok_or(format!("No screenshot available for monitor {}", monitor_index))?;

    // Декодируем base64 в PNG данные
    let png_data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_screenshot)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // Декодируем PNG в raw данные
    let decoder = png::Decoder::new(&png_data[..]);
    let mut reader = decoder.read_info()
        .map_err(|e| format!("Failed to read PNG info: {}", e))?;

    let mut full_rgba_data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut full_rgba_data)
        .map_err(|e| format!("Failed to decode PNG: {}", e))?;

    let full_width = info.width;
    let full_height = info.height;

    println!("Original screenshot size: {}x{}", full_width, full_height);

    // Проверяем границы
    if x + width > full_width || y + height > full_height {
        return Err(format!("Selection area out of bounds: {}x{} at ({},{}) vs image {}x{}",
            width, height, x, y, full_width, full_height));
    }

    // Обрезаем изображение
    let mut cropped_data = Vec::with_capacity((width * height * 4) as usize);

    for row in y..(y + height) {
        let start_idx = ((row * full_width + x) * 4) as usize;
        let end_idx = start_idx + (width * 4) as usize;
        cropped_data.extend_from_slice(&full_rgba_data[start_idx..end_idx]);
    }

    // Кодируем обрезанное изображение в PNG
    let mut result_png_data = Vec::new();
    {
        let w = BufWriter::new(&mut result_png_data);
        let mut encoder = Encoder::new(w, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()
            .map_err(|e| format!("Failed to write PNG header: {}", e))?;

        writer.write_image_data(&cropped_data)
            .map_err(|e| format!("Failed to write PNG data: {}", e))?;
    }

    // Конвертируем в base64
    let base64_image = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &result_png_data);

    println!("Area screenshot cut successfully from saved screenshot");
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

// Команда для сохранения OpenAI API ключа
#[tauri::command]
fn save_openai_api_key(api_key: String) -> Result<(), String> {
    let mut state = load_state();
    state.openai_api_key = Some(api_key);
    save_state(&state);
    println!("OpenAI API key saved");
    Ok(())
}

// Команда для получения сохраненного OpenAI API ключа
#[tauri::command]
fn get_openai_api_key() -> Result<Option<String>, String> {
    let state = load_state();
    Ok(state.openai_api_key)
}

// Команда для сохранения Anthropic API ключа
#[tauri::command]
fn save_anthropic_api_key(api_key: String) -> Result<(), String> {
    let mut state = load_state();
    state.anthropic_api_key = Some(api_key);
    save_state(&state);
    Ok(())
}

// Команда для получения сохраненного Anthropic API ключа
#[tauri::command]
fn get_anthropic_api_key() -> Result<Option<String>, String> {
    let state = load_state();
    Ok(state.anthropic_api_key)
}

// Команда для сохранения настройки автооткрытия ссылок
#[tauri::command]
fn save_auto_open_links(enabled: bool) -> Result<(), String> {
    let mut state = load_state();
    state.auto_open_links = Some(enabled);
    save_state(&state);
    println!("Auto open links saved: {}", enabled);
    Ok(())
}

// Команда для получения настройки автооткрытия ссылок
#[tauri::command]
fn get_auto_open_links() -> Result<Option<bool>, String> {
    let state = load_state();
    Ok(state.auto_open_links)
}

// Команда для открытия URL в браузере
#[tauri::command]
async fn open_url_in_browser(url: String) -> Result<(), String> {
    use std::process::Command;
    #[cfg(unix)]
    use std::os::unix::process::CommandExt;

    println!("Opening URL in browser: {}", url);

    // Добавляем https:// если протокол не указан
    let full_url = if url.starts_with("http://") || url.starts_with("https://") {
        url
    } else {
        format!("https://{}", url)
    };

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&full_url)
            .process_group(0)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(&["/c", "start", "", &full_url])
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&full_url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    println!("URL opened successfully");
    Ok(())
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

// Команда для отправки изображения в Claude
#[tauri::command]
async fn send_to_claude(api_key: String, image_base64: String, prompt: String) -> Result<String, String> {
    use reqwest;

    println!("Sending to Claude...");

    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 1024,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": image_base64
                        }
                    },
                    {
                        "type": "text",
                        "text": prompt
                    }
                ]
            }
        ]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Claude API error: {}", error_text));
    }

    let response_json: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let content = response_json["content"][0]["text"]
        .as_str()
        .ok_or("No content in response")?
        .to_string();

    println!("Claude response received");
    Ok(content)
}

// Команда для эмуляции нажатий клавиш
#[tauri::command]
async fn type_text(text: String) -> Result<(), String> {
    use enigo::{Enigo, Keyboard, Settings};

    println!("Starting to type text: {}", text);

    // Создаём enigo в отдельном потоке, так как он требует синхронного API
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| format!("Failed to create Enigo: {:?}", e))?;

        // Небольшая задержка между символами для надёжности
        for ch in text.chars() {
            enigo.text(&ch.to_string())
                .map_err(|e| format!("Failed to type character '{}': {:?}", ch, e))?;
            std::thread::sleep(Duration::from_millis(10));
        }

        println!("Finished typing text");
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    Ok(())
}

// Команда для переключения DevTools
#[tauri::command]
fn toggle_devtools(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("main") {
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
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

// Команда для открытия терминала с SSH командой
#[tauri::command]
async fn open_terminal(command: String) -> Result<(), String> {
    use tokio::process::Command;

    println!("Opening terminal with command: {}", command);

    #[cfg(target_os = "linux")]
    {
        // Создаем команды заранее для правильного времени жизни
        let cmd1 = format!("{}; exec bash", command);
        let cmd2 = format!("bash -c '{}; exec bash'", command);

        // Пробуем разные терминалы Linux
        let terminals: Vec<(&str, Vec<&str>)> = vec![
            ("gnome-terminal", vec!["--", "bash", "-c", &cmd1]),
            ("konsole", vec!["-e", "bash", "-c", &cmd1]),
            ("xfce4-terminal", vec!["-e", &cmd2]),
            ("xterm", vec!["-e", &cmd2]),
        ];

        for (terminal, args) in terminals {
            match Command::new(terminal)
                .args(&args)
                .spawn()
            {
                Ok(_) => {
                    println!("Successfully opened {} terminal", terminal);
                    return Ok(());
                }
                Err(_) => continue,
            }
        }

        return Err("No terminal emulator found. Please install gnome-terminal, konsole, xfce4-terminal, or xterm.".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        // Windows Terminal или CMD
        match Command::new("cmd")
            .args(&["/c", "start", "cmd", "/k", &command])
            .spawn()
        {
            Ok(_) => {
                println!("Successfully opened Windows terminal");
                return Ok(());
            }
            Err(e) => return Err(format!("Failed to open terminal: {}", e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS Terminal
        let script = format!(
            r#"tell application "Terminal"
                activate
                do script "{}"
            end tell"#,
            command.replace("\"", "\\\"")
        );

        match Command::new("osascript")
            .args(&["-e", &script])
            .spawn()
        {
            Ok(_) => {
                println!("Successfully opened macOS terminal");
                return Ok(());
            }
            Err(e) => return Err(format!("Failed to open terminal: {}", e)),
        }
    }
}

// Структура для JetBrains проекта
#[derive(Serialize, Deserialize, Debug, Clone)]
struct JetBrainsProject {
    ide_name: String,
    ide_version: String,
    project_path: String,
    display_name: String,
    frame_title: String,
    activation_time: Option<String>,
    exists: bool,
}

// Команда для получения списка проектов JetBrains
#[tauri::command]
async fn get_jetbrains_projects() -> Result<Vec<JetBrainsProject>, String> {
    use tokio::process::Command;
    use std::env;

    println!("Getting JetBrains projects...");

    // Получаем домашнюю директорию пользователя
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map_err(|_| "Failed to get home directory".to_string())?;

    // Создаем временный файл для JSON в home директории
    let json_path = format!("{}/.cache/jetbrains_projects.json", home_dir);
    let cache_dir = format!("{}/.cache", home_dir);

    // Создаем директорию .cache если её нет
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    println!("Using cache path: {}", json_path);

    // Находим путь к Python скрипту
    let script_path = if cfg!(debug_assertions) {
        // В режиме разработки используем путь из исходников
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("jetbrains_projects.py")
    } else {
        // В production ищем скрипт рядом с исполняемым файлом
        let exe_path = env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?;
        exe_path.parent()
            .unwrap()
            .join("jetbrains_projects.py")
    };

    println!("Using script path: {:?}", script_path);

    // Запускаем Python скрипт
    let output = Command::new("python3")
        .arg(&script_path)
        .env("JSON_OUTPUT_PATH", &json_path)
        .output()
        .await
        .map_err(|e| format!("Failed to run script: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Script stderr: {}", stderr);
        println!("Script stdout: {}", stdout);
        return Err(format!("Script failed: {}", stderr));
    }

    // Читаем JSON файл
    let json_content = tokio::fs::read_to_string(&json_path)
        .await
        .map_err(|e| format!("Failed to read JSON from {}: {}", json_path, e))?;

    let projects: Vec<JetBrainsProject> = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    println!("Found {} projects", projects.len());
    Ok(projects)
}

// Команда для открытия проекта в JetBrains IDE
#[tauri::command]
async fn open_jetbrains_project(project_path: String, ide_name: String) -> Result<(), String> {
    use std::process::Command;
    #[cfg(unix)]
    use std::os::unix::process::CommandExt;

    println!("Opening project {} in {}", project_path, ide_name);

    // Определяем команду запуска IDE
    let ide_command = match ide_name.to_lowercase().as_str() {
        "rider" => "rider",
        "phpstorm" => "phpstorm",
        "pycharm" => "pycharm",
        "webstorm" => "webstorm",
        "goland" => "goland",
        "rustrover" => "rustrover",
        "datagrip" => "datagrip",
        "clion" => "clion",
        "intellijidea" => "idea",
        _ => return Err(format!("Unknown IDE: {}", ide_name)),
    };

    // Запускаем IDE с проектом как независимый процесс
    #[cfg(unix)]
    {
        Command::new(ide_command)
            .arg(&project_path)
            .process_group(0)  // Создаём новую группу процессов
            .spawn()
            .map_err(|e| format!("Failed to open project: {}", e))?;
    }

    #[cfg(not(unix))]
    {
        Command::new(ide_command)
            .arg(&project_path)
            .spawn()
            .map_err(|e| format!("Failed to open project: {}", e))?;
    }

    println!("Project opened successfully");
    Ok(())
}

// Команда для конвертации видео в MP4
#[tauri::command]
async fn convert_to_mp4(input_path: String) -> Result<String, String> {
    use tokio::process::Command;
    use std::path::Path;

    println!("Converting to MP4: {}", input_path);

    // Проверяем что файл существует
    if !Path::new(&input_path).exists() {
        return Err("Входной файл не найден".to_string());
    }

    // Генерируем путь для выходного файла
    let input_path_obj = Path::new(&input_path);
    let output_path = input_path_obj
        .with_extension("mp4")
        .to_string_lossy()
        .to_string();

    println!("Output path: {}", output_path);

    // Проверяем наличие ffmpeg
    let ffmpeg_check = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await;

    if ffmpeg_check.is_err() {
        return Err("FFmpeg не установлен. Установите FFmpeg для конвертации видео.".to_string());
    }

    // Запускаем конвертацию
    let output = Command::new("ffmpeg")
        .args(&[
            "-i", &input_path,
            "-c:v", "libx264",    // H.264 видео кодек
            "-crf", "23",         // Качество (0-51, меньше = лучше)
            "-c:a", "aac",        // AAC аудио кодек
            "-b:a", "192k",       // Битрейт аудио
            "-y",                 // Перезаписать без запроса
            &output_path
        ])
        .output()
        .await
        .map_err(|e| format!("Ошибка запуска FFmpeg: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("FFmpeg error: {}", stderr);
        return Err(format!("Ошибка конвертации: {}", stderr));
    }

    println!("Conversion completed successfully");
    Ok(output_path)
}

// Команда для начала записи кликов
#[tauri::command]
async fn start_click_recording(state: tauri::State<'_, ClickRecordingState>) -> Result<(), String> {
    println!("Starting click recording...");

    // Очищаем предыдущую запись
    {
        let mut clicks = state.clicks.lock().unwrap();
        clicks.clear();
    }

    // Устанавливаем флаг записи
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = true;
    }

    STOP_RECORDING.store(false, std::sync::atomic::Ordering::SeqCst);

    // Клонируем Arc для передачи в поток
    let clicks_arc = state.clicks.clone();

    tokio::task::spawn_blocking(move || {
        use rdev::{listen, Event, EventType};

        let callback = move |event: Event| {
            if STOP_RECORDING.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }

            // Записываем левые и правые клики мыши
            let button = match event.event_type {
                EventType::ButtonPress(rdev::Button::Left) => Some("left"),
                EventType::ButtonPress(rdev::Button::Right) => Some("right"),
                _ => None,
            };

            if let Some(btn) = button {
                // Получаем позицию из события rdev (более точные координаты)
                let (x, y) = {
                    // Используем xdotool для более точных координат на Linux
                    #[cfg(target_os = "linux")]
                    {
                        use std::process::Command;
                        let output = Command::new("xdotool")
                            .args(&["getmouselocation", "--shell"])
                            .output();

                        if let Ok(output) = output {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let mut x_val = 0i32;
                            let mut y_val = 0i32;

                            for line in stdout.lines() {
                                if line.starts_with("X=") {
                                    x_val = line[2..].parse().unwrap_or(0);
                                } else if line.starts_with("Y=") {
                                    y_val = line[2..].parse().unwrap_or(0);
                                }
                            }
                            (x_val, y_val)
                        } else {
                            // Fallback к enigo
                            use enigo::{Enigo, Mouse, Settings};
                            match Enigo::new(&Settings::default()) {
                                Ok(enigo) => enigo.location().unwrap_or((0, 0)),
                                Err(_) => (0, 0),
                            }
                        }
                    }

                    #[cfg(not(target_os = "linux"))]
                    {
                        use enigo::{Enigo, Mouse, Settings};
                        match Enigo::new(&Settings::default()) {
                            Ok(enigo) => enigo.location().unwrap_or((0, 0)),
                            Err(_) => (0, 0),
                        }
                    }
                };

                let click = ClickPoint {
                    x,
                    y,
                    monitor: 0,
                    button: btn.to_string(),
                };
                println!("Click recorded: x={}, y={}, button={}", x, y, btn);

                if let Ok(mut clicks_lock) = clicks_arc.lock() {
                    clicks_lock.push(click);
                }
            }
        };

        // Слушаем события до остановки
        if let Err(error) = listen(callback) {
            println!("Error listening for clicks: {:?}", error);
        }
    });

    Ok(())
}

// Команда для остановки записи кликов
#[tauri::command]
fn stop_click_recording(state: tauri::State<'_, ClickRecordingState>) -> Result<Vec<ClickPoint>, String> {
    println!("Stopping click recording...");

    // Останавливаем запись
    STOP_RECORDING.store(true, std::sync::atomic::Ordering::SeqCst);

    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = false;
    }

    // Возвращаем записанные клики
    let clicks = state.clicks.lock().unwrap();
    let result = clicks.clone();

    println!("Recording stopped, {} clicks captured", result.len());
    Ok(result)
}

// Команда для воспроизведения последовательности кликов
#[tauri::command]
async fn play_click_sequence(clicks: Vec<ClickPoint>, interval_ms: u64, repeat_count: u32) -> Result<(), String> {
    println!("Playing {} clicks with {}ms interval, {} repeat(s)...", clicks.len(), interval_ms, repeat_count);

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        for repeat in 0..repeat_count {
            if repeat_count > 1 {
                println!("=== Repeat {}/{} ===", repeat + 1, repeat_count);
            }

            // Используем xdotool на Linux для более точных кликов
            #[cfg(target_os = "linux")]
            {
                use std::process::Command;

                // Получаем текущую позицию курсора
                let mut current_x = 0i32;
                let mut current_y = 0i32;

                let output = Command::new("xdotool")
                    .args(&["getmouselocation", "--shell"])
                    .output();

                if let Ok(output) = output {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if line.starts_with("X=") {
                            current_x = line[2..].parse().unwrap_or(0);
                        } else if line.starts_with("Y=") {
                            current_y = line[2..].parse().unwrap_or(0);
                        }
                    }
                }

                for (i, click) in clicks.iter().enumerate() {
                    // Плавное перемещение курсора (медленно для хорошей видимости)
                    let steps = 100; // количество шагов для плавности
                    let dx = (click.x - current_x) as f64 / steps as f64;
                    let dy = (click.y - current_y) as f64 / steps as f64;

                    println!("Moving cursor from ({}, {}) to ({}, {})", current_x, current_y, click.x, click.y);

                    for step in 1..=steps {
                        let intermediate_x = current_x + (dx * step as f64) as i32;
                        let intermediate_y = current_y + (dy * step as f64) as i32;

                        let _ = Command::new("xdotool")
                            .args(&["mousemove", &intermediate_x.to_string(), &intermediate_y.to_string()])
                            .status();

                        std::thread::sleep(Duration::from_millis(10));
                    }

                    // Финальная позиция
                    let _ = Command::new("xdotool")
                        .args(&["mousemove", &click.x.to_string(), &click.y.to_string()])
                        .status();

                    std::thread::sleep(Duration::from_millis(50));

                    // Кликаем нужной кнопкой
                    let button_num = if click.button == "right" { "3" } else { "1" };
                    let result = Command::new("xdotool")
                        .arg("click")
                        .arg(button_num)
                        .status();

                    if let Err(e) = result {
                        return Err(format!("Failed to click: {}", e));
                    }

                    println!("Click {} ({}) at ({}, {})", i + 1, click.button, click.x, click.y);

                    current_x = click.x;
                    current_y = click.y;

                    // Задержка между кликами
                    if i < clicks.len() - 1 {
                        std::thread::sleep(Duration::from_millis(interval_ms));
                    }
                }
            }

            #[cfg(not(target_os = "linux"))]
            {
                use enigo::{Enigo, Mouse, Button, Coordinate, Settings};

                let mut enigo = Enigo::new(&Settings::default())
                    .map_err(|e| format!("Failed to create Enigo: {:?}", e))?;

                // Получаем текущую позицию
                let (mut current_x, mut current_y) = enigo.location().unwrap_or((0, 0));

                for (i, click) in clicks.iter().enumerate() {
                    // Плавное перемещение курсора
                    let steps = 20;
                    let dx = (click.x - current_x) as f64 / steps as f64;
                    let dy = (click.y - current_y) as f64 / steps as f64;

                    for step in 1..=steps {
                        let intermediate_x = current_x + (dx * step as f64) as i32;
                        let intermediate_y = current_y + (dy * step as f64) as i32;

                        let _ = enigo.move_mouse(intermediate_x, intermediate_y, Coordinate::Abs);
                        std::thread::sleep(Duration::from_millis(5));
                    }

                    // Финальная позиция
                    enigo.move_mouse(click.x, click.y, Coordinate::Abs)
                        .map_err(|e| format!("Failed to move mouse: {:?}", e))?;

                    std::thread::sleep(Duration::from_millis(50));

                    // Кликаем нужной кнопкой
                    let btn = if click.button == "right" { Button::Right } else { Button::Left };
                    enigo.button(btn, enigo::Direction::Click)
                        .map_err(|e| format!("Failed to click: {:?}", e))?;

                    println!("Click {} ({}) at ({}, {})", i + 1, click.button, click.x, click.y);

                    current_x = click.x;
                    current_y = click.y;

                    // Задержка между кликами
                    if i < clicks.len() - 1 {
                        std::thread::sleep(Duration::from_millis(interval_ms));
                    }
                }
            }

            // Задержка между повторениями
            if repeat < repeat_count - 1 {
                std::thread::sleep(Duration::from_millis(interval_ms));
            }
        }

        println!("Click sequence completed ({} repeats)", repeat_count);
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    Ok(())
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
        .plugin(tauri_plugin_dialog::init())
        .manage(ScreenshotState {
            data: Mutex::new(HashMap::new()),
        })
        .manage(ClickRecordingState {
            is_recording: Mutex::new(false),
            clicks: std::sync::Arc::new(Mutex::new(Vec::new())),
        })
        .manage(PopupState {
            image_data: Mutex::new(None),
            screen_x: Mutex::new(0),
            screen_y: Mutex::new(0),
        })
        .setup(|app| {
            // F12, F5, F11 теперь обрабатываются на фронтенде, а не глобально

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

            // CommandOrControl+Shift+C теперь обрабатывается на фронтенде, а не глобально

            // Регистрируем Ctrl+PrintScreen для переключения на страницу переводов и захвата скриншота
            match app.global_shortcut().on_shortcut("Ctrl+PrintScreen", move |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    println!("Ctrl+PrintScreen pressed - switching to translations and capturing screenshot");
                    if let Some(window) = app.get_webview_window("main") {
                        // Показываем окно и отправляем событие для переключения на страницу и захвата
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.eval("window.dispatchEvent(new CustomEvent('translation-hotkey-pressed'))");
                    }
                }
            }) {
                Ok(_) => println!("Ctrl+PrintScreen shortcut registered successfully"),
                Err(e) => eprintln!("Failed to register Ctrl+PrintScreen shortcut: {}", e),
            }

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

                // Отключаем зум в WebView2
                #[cfg(target_os = "windows")]
                {
                    println!("Disabling zoom in WebView2...");
                    let _ = window.with_webview(|webview| {
                        #[cfg(target_os = "windows")]
                        unsafe {
                            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller;
                            use windows::core::Interface;

                            let controller = webview.controller();
                            if let Ok(controller) = controller.cast::<ICoreWebView2Controller>() {
                                // Устанавливаем зум на 1.0 и блокируем его изменение
                                let _ = controller.SetZoomFactor(1.0);
                                println!("Zoom factor set to 1.0");
                            }
                        }
                        Ok(())
                    });
                }
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
            open_translation_popup,
            get_popup_screenshot,
            get_popup_screen_position,
            close_translation_popup,
            solve_and_click,
            move_translation_popup,
            get_translation_popup_position,
            get_window_size,
            set_window_size,
            save_translation_hotkey,
            get_translation_hotkey,
            save_last_route,
            get_last_route,
            save_openai_api_key,
            get_openai_api_key,
            save_anthropic_api_key,
            get_anthropic_api_key,
            save_auto_open_links,
            get_auto_open_links,
            open_url_in_browser,
            send_to_chatgpt,
            send_to_claude,
            type_text,
            toggle_devtools,
            open_terminal,
            convert_to_mp4,
            get_jetbrains_projects,
            open_jetbrains_project,
            start_click_recording,
            stop_click_recording,
            play_click_sequence
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

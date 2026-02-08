use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use std::time::Duration;
use sysinfo::Components;
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use screenshots::Screen;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use portable_pty::{CommandBuilder, PtySize, native_pty_system, MasterPty, Child};
use std::io::{Read, Write};

mod websocket_stream;

// PTY Session management
struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

static PTY_SESSIONS: Lazy<Mutex<HashMap<String, PtySession>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

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

// Команда для захвата скриншота конкретного монитора
#[tauri::command]
async fn capture_monitor_screenshot(monitor_index: usize) -> Result<String, String> {
    use png::Encoder;
    use png::ColorType;
    use std::io::BufWriter;

    println!("Capturing screenshot of monitor {}...", monitor_index);

    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    if screens.is_empty() {
        return Err("No screens found".to_string());
    }

    if monitor_index >= screens.len() {
        return Err(format!("Monitor index {} out of range (available: {})", monitor_index, screens.len()));
    }

    let screen = &screens[monitor_index];
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

    println!("Screenshot of monitor {} captured successfully", monitor_index);
    Ok(base64_image)
}

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

    // Увеличенная пауза для полного завершения уничтожения окон
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Проверяем, что все окна действительно закрыты
    for monitor_index in 0..10 {
        let window_label = format!("area-selector-{}", monitor_index);
        let mut attempts = 0;
        while app_handle.get_webview_window(&window_label).is_some() && attempts < 10 {
            println!("Waiting for window {} to be fully destroyed...", window_label);
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            attempts += 1;
        }
    }

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
            println!("Destroying window: {}", window_label);
            match window.destroy() {
                Ok(_) => closed_count += 1,
                Err(e) => println!("Warning: Failed to destroy window {}: {}", window_label, e),
            }
            monitor_index += 1;
        } else {
            break;
        }
    }
    if closed_count > 0 {
        println!("Destroyed {} area-selector window(s)", closed_count);
        // Увеличенная пауза для полного завершения уничтожения окон
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
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
    if let Some(existing) = app_handle.get_webview_window("screenshot-popup") {
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
        "screenshot-popup",
        WebviewUrl::App("/index.html#/screenshot-popup".into())
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
    if let Some(window) = app_handle.get_webview_window("screenshot-popup") {
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
    if let Some(window) = app_handle.get_webview_window("screenshot-popup") {
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

        // Плавное перемещение курсора (быстрее в 3 раза)
        let steps = 17; // Уменьшено с 50 до ~17 (в 3 раза меньше)
        let dx = (x - current_x) as f64 / steps as f64;
        let dy = (y - current_y) as f64 / steps as f64;

        println!("Moving cursor from ({}, {}) to ({}, {})", current_x, current_y, x, y);

        for step in 1..=steps {
            let intermediate_x = current_x + (dx * step as f64) as i32;
            let intermediate_y = current_y + (dy * step as f64) as i32;

            let _ = Command::new("xdotool")
                .args(&["mousemove", &intermediate_x.to_string(), &intermediate_y.to_string()])
                .status();

            std::thread::sleep(Duration::from_millis(3)); // Уменьшено с 10 до 3 мс
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

// Команда для выполнения клика с различными кнопками и количеством кликов
#[tauri::command]
async fn perform_click(x: i32, y: i32, button: String, click_count: u32) -> Result<(), String> {
    use std::process::Command;

    println!("Performing {} click(s) with {} button at ({}, {})", click_count, button, x, y);

    tokio::task::spawn_blocking(move || -> Result<(), String> {
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
        let steps = 17;
        let dx = (x - current_x) as f64 / steps as f64;
        let dy = (y - current_y) as f64 / steps as f64;

        for step in 1..=steps {
            let intermediate_x = current_x + (dx * step as f64) as i32;
            let intermediate_y = current_y + (dy * step as f64) as i32;

            let _ = Command::new("xdotool")
                .args(&["mousemove", &intermediate_x.to_string(), &intermediate_y.to_string()])
                .status();

            std::thread::sleep(Duration::from_millis(3));
        }

        // Финальная позиция
        let _ = Command::new("xdotool")
            .args(&["mousemove", &x.to_string(), &y.to_string()])
            .status();

        std::thread::sleep(Duration::from_millis(100));

        // Определяем номер кнопки для xdotool
        let button_num = match button.as_str() {
            "left" => "1",
            "middle" => "2",
            "right" => "3",
            _ => "1",
        };

        // Выполняем клики
        for _ in 0..click_count {
            let _ = Command::new("xdotool")
                .arg("click")
                .arg(button_num)
                .status();

            // Небольшая задержка между кликами для двойного клика
            if click_count > 1 {
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        println!("{} click(s) performed at ({}, {}) with {} button", click_count, x, y, button);

        Ok(())
    }).await.map_err(|e| format!("Task failed: {}", e))?
}

// Команда для перемещения popup окна
#[tauri::command]
async fn move_translation_popup(app_handle: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("screenshot-popup") {
        window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
            .map_err(|e| format!("Failed to move popup: {}", e))?;
    }
    Ok(())
}

// Команда для получения позиции popup окна
#[tauri::command]
async fn get_translation_popup_position(app_handle: tauri::AppHandle) -> Result<(i32, i32), String> {
    if let Some(window) = app_handle.get_webview_window("screenshot-popup") {
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
    if let Some(window) = app_handle.get_webview_window("screenshot-popup") {
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
    if let Some(window) = app_handle.get_webview_window("screenshot-popup") {
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
                    // Плавное перемещение курсора (быстрее в 3 раза)
                    let steps = 33; // Уменьшено с 100 до ~33 (в 3 раза меньше)
                    let dx = (click.x - current_x) as f64 / steps as f64;
                    let dy = (click.y - current_y) as f64 / steps as f64;

                    println!("Moving cursor from ({}, {}) to ({}, {})", current_x, current_y, click.x, click.y);

                    for step in 1..=steps {
                        let intermediate_x = current_x + (dx * step as f64) as i32;
                        let intermediate_y = current_y + (dy * step as f64) as i32;

                        let _ = Command::new("xdotool")
                            .args(&["mousemove", &intermediate_x.to_string(), &intermediate_y.to_string()])
                            .status();

                        std::thread::sleep(Duration::from_millis(3)); // Уменьшено с 10 до 3 мс
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
                    // Плавное перемещение курсора (быстрее в 3 раза)
                    let steps = 7; // Уменьшено с 20 до ~7 (в 3 раза меньше)
                    let dx = (click.x - current_x) as f64 / steps as f64;
                    let dy = (click.y - current_y) as f64 / steps as f64;

                    for step in 1..=steps {
                        let intermediate_x = current_x + (dx * step as f64) as i32;
                        let intermediate_y = current_y + (dy * step as f64) as i32;

                        let _ = enigo.move_mouse(intermediate_x, intermediate_y, Coordinate::Abs);
                        std::thread::sleep(Duration::from_millis(2)); // Уменьшено с 5 до 2 мс
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

// Структура для управления стриминг сервером
struct StreamingServer {
    handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    stop_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,
    screen_index: Mutex<usize>,
}

impl StreamingServer {
    fn new() -> Self {
        Self {
            handle: Mutex::new(None),
            stop_signal: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            screen_index: Mutex::new(0),
        }
    }
}

// Команда для получения локального IP адреса
#[tauri::command]
fn get_local_ip() -> Result<String, String> {
    use local_ip_address::local_ip;

    match local_ip() {
        Ok(ip) => Ok(ip.to_string()),
        Err(e) => Err(format!("Failed to get local IP: {}", e)),
    }
}

// Структура для ответа start_screen_streaming
#[derive(Serialize)]
struct StreamingInfo {
    ip: String,
    port: u16,
}

// Команда для запуска стриминг сервера
#[tauri::command]
fn start_screen_streaming(port: u16, screen_index: usize, state: tauri::State<'_, StreamingServer>) -> Result<StreamingInfo, String> {
    use local_ip_address::local_ip;

    println!("Starting WebSocket streaming server on port {} for screen {}", port, screen_index);

    // Получаем локальный IP
    let ip = match local_ip() {
        Ok(ip) => ip.to_string(),
        Err(e) => return Err(format!("Failed to get local IP: {}", e)),
    };

    // Останавливаем предыдущий сервер если он был запущен
    {
        let mut handle = state.handle.lock().unwrap();
        if handle.is_some() {
            state.stop_signal.store(true, std::sync::atomic::Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(200));
            *handle = None;
        }
    }

    // Сбрасываем флаг остановки
    state.stop_signal.store(false, std::sync::atomic::Ordering::SeqCst);

    // Сохраняем индекс экрана
    {
        let mut idx = state.screen_index.lock().unwrap();
        *idx = screen_index;
    }

    // Запускаем WebSocket сервер
    let stop_signal = state.stop_signal.clone();
    let handle = websocket_stream::start_websocket_server(port, screen_index, stop_signal)?;

    // Сохраняем handle
    {
        let mut h = state.handle.lock().unwrap();
        *h = Some(handle);
    }

    println!("WebSocket streaming server started on {}:{}", ip, port);
    Ok(StreamingInfo { ip, port })
}


// Команда для остановки стриминг сервера
#[tauri::command]
fn stop_screen_streaming(state: tauri::State<'_, StreamingServer>) -> Result<(), String> {
    println!("Stopping streaming server...");

    // Устанавливаем флаг остановки
    state.stop_signal.store(true, std::sync::atomic::Ordering::SeqCst);

    // Ждём завершения потока
    {
        let mut handle = state.handle.lock().unwrap();
        if let Some(h) = handle.take() {
            // Не используем join, так как это может заблокировать основной поток
            // Вместо этого просто отсоединяем поток
            drop(h);
        }
    }

    println!("Streaming server stopped");
    Ok(())
}

// Структура для результата поиска изображения
#[derive(Serialize, Deserialize, Clone)]
struct ImageSearchResult {
    x: i32,
    y: i32,
    match_percentage: f64,
}

// Команда для поиска изображения на экране (оптимизированная версия)
#[tauri::command]
async fn find_image_on_screen(
    screen_image: String,
    target_image: String,
    _target_width: u32,
    _target_height: u32,
) -> Result<Option<ImageSearchResult>, String> {
    use image::GenericImageView;
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    println!("Searching for image on screen (optimized)...");
    let start_time = std::time::Instant::now();

    // Декодируем base64 изображения
    let screen_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &screen_image)
        .map_err(|e| format!("Failed to decode screen image: {}", e))?;
    let target_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &target_image)
        .map_err(|e| format!("Failed to decode target image: {}", e))?;

    // Загружаем изображения
    let screen = image::load_from_memory(&screen_bytes)
        .map_err(|e| format!("Failed to load screen image: {}", e))?;
    let target = image::load_from_memory(&target_bytes)
        .map_err(|e| format!("Failed to load target image: {}", e))?;

    let screen_rgba = screen.to_rgba8();
    let target_rgba = target.to_rgba8();

    let (screen_w, screen_h) = screen.dimensions();
    let (target_w, target_h) = target.dimensions();

    println!("Screen size: {}x{}, Target size: {}x{}", screen_w, screen_h, target_w, target_h);

    if target_w > screen_w || target_h > screen_h {
        return Ok(None);
    }

    let threshold = 50i32; // Порог различия для каждого канала цвета
    let step = 2; // Шаг поиска - проверяем каждую 2-ю позицию для ускорения

    // Используем Arc для безопасного доступа к изображениям из разных потоков
    let screen_data = Arc::new(screen_rgba);
    let target_data = Arc::new(target_rgba);
    let found = Arc::new(AtomicBool::new(false));

    // Проверяем с разными порогами от 100% до 60%
    let match_thresholds = [1.00, 0.95, 0.90, 0.85, 0.80, 0.75, 0.70, 0.65, 0.60];

    for &match_threshold in match_thresholds.iter() {
        if found.load(Ordering::Relaxed) {
            break;
        }

        println!("Searching with threshold {:.0}%...", match_threshold * 100.0);

        // Параллельный поиск по строкам экрана
        let y_positions: Vec<u32> = (0..=(screen_h - target_h)).step_by(step as usize).collect();
        let search_result = y_positions
            .par_iter()
            .find_map_any(|&y| {
                // Если уже нашли, прекращаем поиск
                if found.load(Ordering::Relaxed) {
                    return None;
                }

                for x in (0..=(screen_w - target_w)).step_by(step as usize) {
                    if found.load(Ordering::Relaxed) {
                        return None;
                    }

                    // Быстрая проверка: сравниваем только ключевые точки (углы и центр)
                    let key_points = [
                        (0, 0),
                        (target_w / 2, target_h / 2),
                        (target_w - 1, 0),
                        (0, target_h - 1),
                        (target_w - 1, target_h - 1),
                    ];

                    let mut key_match = true;
                    for (tx, ty) in key_points.iter() {
                        let screen_pixel = screen_data.get_pixel(x + tx, y + ty);
                        let target_pixel = target_data.get_pixel(*tx, *ty);

                        if target_pixel[3] < 128 {
                            continue;
                        }

                        let r_diff = (screen_pixel[0] as i32 - target_pixel[0] as i32).abs();
                        let g_diff = (screen_pixel[1] as i32 - target_pixel[1] as i32).abs();
                        let b_diff = (screen_pixel[2] as i32 - target_pixel[2] as i32).abs();

                        if r_diff > threshold || g_diff > threshold || b_diff > threshold {
                            key_match = false;
                            break;
                        }
                    }

                    if !key_match {
                        continue;
                    }

                    // Если ключевые точки совпали, проверяем всё изображение с прореживанием
                    let mut matched_pixels = 0;
                    let mut total_checked = 0;
                    let check_step = 2; // Проверяем каждый 2-й пиксель

                    for ty in (0..target_h).step_by(check_step as usize) {
                        for tx in (0..target_w).step_by(check_step as usize) {
                            let screen_pixel = screen_data.get_pixel(x + tx, y + ty);
                            let target_pixel = target_data.get_pixel(tx, ty);

                            if target_pixel[3] < 128 {
                                matched_pixels += 1;
                                total_checked += 1;
                                continue;
                            }

                            let r_diff = (screen_pixel[0] as i32 - target_pixel[0] as i32).abs();
                            let g_diff = (screen_pixel[1] as i32 - target_pixel[1] as i32).abs();
                            let b_diff = (screen_pixel[2] as i32 - target_pixel[2] as i32).abs();

                            if r_diff <= threshold && g_diff <= threshold && b_diff <= threshold {
                                matched_pixels += 1;
                            }
                            total_checked += 1;
                        }
                    }

                    let match_rate = matched_pixels as f64 / total_checked as f64;

                    if match_rate >= match_threshold {
                        found.store(true, Ordering::Relaxed);
                        println!("Image found at ({}, {}) with match rate: {:.2}% in {:.2}s",
                            x, y, match_rate * 100.0, start_time.elapsed().as_secs_f64());
                        return Some(ImageSearchResult {
                            x: x as i32,
                            y: y as i32,
                            match_percentage: match_rate * 100.0,
                        });
                    }
                }
                None
            });

        if let Some(res) = search_result {
            return Ok(Some(res));
        }
    }

    let elapsed = start_time.elapsed();
    println!("Image not found on screen. Search took {:.2}s", elapsed.as_secs_f64());
    Ok(None)
}

// ==================== PTY Terminal Commands ====================

#[derive(Serialize, Clone)]
struct PtyOutputEvent {
    session_id: String,
    data: String,
}

#[derive(Serialize, Clone)]
struct PtyExitEvent {
    session_id: String,
    exit_code: Option<u32>,
}

#[tauri::command]
fn create_pty_session(
    rows: u16,
    cols: u16,
    working_directory: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    use tauri::Emitter;

    let session_id = format!("pty-{}", uuid::Uuid::new_v4().to_string());

    let pty_system = native_pty_system();

    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| format!("Failed to open PTY: {}", e))?;

    // Build shell command
    #[cfg(target_os = "windows")]
    let mut cmd = CommandBuilder::new("cmd.exe");

    #[cfg(not(target_os = "windows"))]
    let mut cmd = CommandBuilder::new("bash");

    // Set working directory
    if let Some(ref dir) = working_directory {
        cmd.cwd(dir);
    }

    // Spawn shell
    let child = pair.slave.spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell: {}", e))?;

    // Get master for reading/writing
    let master = pair.master;

    // Get writer for sending input
    let writer = master.take_writer()
        .map_err(|e| format!("Failed to get writer: {}", e))?;

    // Clone for reader thread
    let session_id_clone = session_id.clone();
    let app_handle_clone = app_handle.clone();

    // Create reader for output
    let mut reader = master.try_clone_reader()
        .map_err(|e| format!("Failed to clone reader: {}", e))?;

    // Spawn thread to read PTY output
    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let event = PtyOutputEvent {
                        session_id: session_id_clone.clone(),
                        data,
                    };
                    let _ = app_handle_clone.emit("pty-output", event);
                }
                Err(_) => break,
            }
        }
    });

    // Store session
    let session = PtySession {
        master,
        writer,
        child,
    };

    PTY_SESSIONS.lock().unwrap().insert(session_id.clone(), session);

    println!("Created PTY session: {}", session_id);
    Ok(session_id)
}

#[tauri::command]
fn write_to_pty(session_id: String, data: String) -> Result<(), String> {
    let mut sessions = PTY_SESSIONS.lock().unwrap();

    if let Some(session) = sessions.get_mut(&session_id) {
        session.writer.write_all(data.as_bytes())
            .map_err(|e| format!("Failed to write to PTY: {}", e))?;
        session.writer.flush()
            .map_err(|e| format!("Failed to flush PTY: {}", e))?;
        Ok(())
    } else {
        Err(format!("Session not found: {}", session_id))
    }
}

#[tauri::command]
fn resize_pty(session_id: String, rows: u16, cols: u16) -> Result<(), String> {
    let sessions = PTY_SESSIONS.lock().unwrap();

    if let Some(session) = sessions.get(&session_id) {
        session.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }).map_err(|e| format!("Failed to resize PTY: {}", e))?;

        Ok(())
    } else {
        Err(format!("Session not found: {}", session_id))
    }
}

#[tauri::command]
fn kill_pty_session(session_id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;

    let mut sessions = PTY_SESSIONS.lock().unwrap();

    if let Some(mut session) = sessions.remove(&session_id) {
        // Try to kill the child process
        let _ = session.child.kill();

        // Get exit code
        let exit_code = session.child.try_wait()
            .ok()
            .flatten()
            .map(|status| status.exit_code());

        // Emit exit event
        let event = PtyExitEvent {
            session_id: session_id.clone(),
            exit_code,
        };
        let _ = app_handle.emit("pty-exit", event);

        println!("Killed PTY session: {}", session_id);
        Ok(())
    } else {
        Err(format!("Session not found: {}", session_id))
    }
}

#[tauri::command]
fn get_pty_sessions() -> Vec<String> {
    PTY_SESSIONS.lock().unwrap().keys().cloned().collect()
}

// ==================== End PTY Terminal Commands ====================

// Структура для события вывода команды
#[derive(Serialize, Clone)]
struct CommandOutputEvent {
    command_id: String,
    data: String,
    stream: String, // "stdout" или "stderr"
}

// Структура для события завершения команды
#[derive(Serialize, Clone)]
struct CommandCompleteEvent {
    command_id: String,
    success: bool,
}

// Команда для выполнения shell команды с потоковым выводом
#[tauri::command]
async fn execute_command_stream(
    command: String,
    command_id: String,
    working_directory: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    use tokio::process::Command;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tauri::Emitter;

    println!("Executing command with streaming: {} (working_dir: {:?})", command, working_directory);

    #[cfg(target_os = "linux")]
    let mut cmd = Command::new("bash");
    #[cfg(target_os = "linux")]
    cmd.arg("-c").arg(&command);

    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("cmd");
    #[cfg(target_os = "windows")]
    cmd.args(&["/C", &command]);

    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("bash");
    #[cfg(target_os = "macos")]
    cmd.arg("-c").arg(&command);

    // Устанавливаем рабочую директорию если она указана
    if let Some(ref working_dir) = working_directory {
        cmd.current_dir(working_dir);
    }

    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to get stderr")?;

    let command_id_clone1 = command_id.clone();
    let command_id_clone2 = command_id.clone();
    let app_handle_clone1 = app_handle.clone();
    let app_handle_clone2 = app_handle.clone();

    // Читаем stdout в отдельной задаче
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let event = CommandOutputEvent {
                command_id: command_id_clone1.clone(),
                data: line + "\n",
                stream: "stdout".to_string(),
            };
            let _ = app_handle_clone1.emit("command-output", event);
        }
    });

    // Читаем stderr в отдельной задаче
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let event = CommandOutputEvent {
                command_id: command_id_clone2.clone(),
                data: line + "\n",
                stream: "stderr".to_string(),
            };
            let _ = app_handle_clone2.emit("command-output", event);
        }
    });

    // Ждём завершения обеих задач
    let _ = stdout_task.await;
    let _ = stderr_task.await;

    // Ждём завершения процесса
    let status = child.wait().await.map_err(|e| format!("Failed to wait for command: {}", e))?;

    // Отправляем событие завершения
    let complete_event = CommandCompleteEvent {
        command_id: command_id.clone(),
        success: status.success(),
    };
    let _ = app_handle.emit("command-complete", complete_event);

    println!("Command {} completed with status: {}", command_id, status.success());
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

// Команда для сохранения шаблонов кнопок
#[tauri::command]
async fn save_button_templates(templates_json: String) -> Result<(), String> {
    let home_dir = std::env::var("HOME").map_err(|_| "Failed to get HOME environment variable")?;
    let app_dir = std::path::PathBuf::from(home_dir).join(".local/share/com.bro.app");

    // Создаём директорию если не существует
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("Failed to create app directory: {}", e))?;

    let templates_path = app_dir.join("button_templates.json");

    std::fs::write(&templates_path, templates_json)
        .map_err(|e| format!("Failed to write templates file: {}", e))?;

    println!("Button templates saved to {:?}", templates_path);
    Ok(())
}

// Команда для загрузки шаблонов кнопок
#[tauri::command]
async fn load_button_templates() -> Result<String, String> {
    let home_dir = std::env::var("HOME").map_err(|_| "Failed to get HOME environment variable")?;
    let templates_path = std::path::PathBuf::from(home_dir).join(".local/share/com.bro.app/button_templates.json");

    if templates_path.exists() {
        let data = std::fs::read_to_string(&templates_path)
            .map_err(|e| format!("Failed to read templates file: {}", e))?;
        Ok(data)
    } else {
        // Возвращаем пустой массив если файл не существует
        Ok("[]".to_string())
    }
}

// Команда для показа оверлейной кнопки
#[tauri::command]
async fn show_overlay_button(
    app_handle: tauri::AppHandle,
    template_id: String,
    template_name: String,
) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;
    use tauri::WebviewUrl;
    use tauri::WindowSizeConstraints;
    use tauri::PixelUnit;
    use tauri::LogicalUnit;


    println!("Showing overlay button for template: {}", template_id);

    // Закрываем существующее окно если оно есть
    if let Some(existing_window) = app_handle.get_webview_window("overlay-button") {
        println!("Destroying existing overlay button window");
        let _ = existing_window.destroy();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Получаем позицию главного окна для определения текущего монитора
    let main_window = app_handle.get_webview_window("main")
        .ok_or("Main window not found")?;

    let main_window_position = main_window.outer_position()
        .map_err(|e| format!("Failed to get main window position: {}", e))?;

    // Получаем все экраны
    use screenshots::Screen;
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    // Находим монитор, на котором находится главное окно
    let current_screen = screens.iter()
        .find(|screen| {
            let display = &screen.display_info;
            main_window_position.x >= display.x &&
            main_window_position.x < display.x + display.width as i32 &&
            main_window_position.y >= display.y &&
            main_window_position.y < display.y + display.height as i32
        })
        .or_else(|| screens.first())
        .ok_or("No screen found")?;

    let _display = &current_screen.display_info;

    // Начальная позиция - центр текущего монитора
    // let initial_x = _display.x as f64 + (_display.width as f64 / 2.0) - 100.0;
    // let initial_y = _display.y as f64 + (_display.height as f64 / 2.0) - 25.0;

    let constraints = WindowSizeConstraints {
        min_width: Some(PixelUnit::Logical(LogicalUnit(100.0))),
        min_height: Some(PixelUnit::Logical(LogicalUnit(30.0))),
        max_width: Some(PixelUnit::Logical(LogicalUnit(100.0))),
        max_height: Some(PixelUnit::Logical(LogicalUnit(30.0))),
    };

    let webview_window = WebviewWindowBuilder::new(
        &app_handle,
        "overlay-button",
        WebviewUrl::App(format!("/index.html#/overlay-button?templateId={}&templateName={}", template_id, template_name).into())
    )
    .title("Overlay Button")
    // .position(initial_x, initial_y)
    .min_inner_size(100.0, 30.0)
    .max_inner_size(100.0, 30.0)
    .inner_size_constraints(constraints)
    .inner_size(100.0, 30.0)
    .decorations(false)
    .transparent(true)
    .always_on_top(false)
    .skip_taskbar(true)
    .visible(true)
    .resizable(true) // keep true to avoid GTK 200px minimum floor
    // .focused(uefalse)
    .devtools(true)
    .build()
    .unwrap();
    // .map_err(|e| format!("Failed to create overlay button window: {}", e))?;

    // Force small size via GTK API
    #[cfg(target_os = "linux")]
    {
        force_gtk_window_size(&webview_window, 100, 30);
    }

    let _ = webview_window.set_always_on_top(true);

    Ok(())
}

// Команда для скрытия оверлейной кнопки
#[tauri::command]
async fn hide_overlay_button(app_handle: tauri::AppHandle) -> Result<(), String> {
    println!("Hiding overlay button...");

    if let Some(window) = app_handle.get_webview_window("overlay-button") {
        println!("Destroying overlay button window");
        match window.destroy() {
            Ok(_) => println!("Overlay button window destroyed"),
            Err(e) => println!("Warning: Failed to destroy overlay button window: {}", e),
        }
    } else {
        println!("No overlay button window to hide");
    }

    // Отправляем событие в главное окно
    if let Some(main_window) = app_handle.get_webview_window("main") {
        let _ = main_window.eval("window.dispatchEvent(new CustomEvent('overlay-button-closed'))");
    }

    Ok(())
}

// Force small window size on Linux by bypassing WebKit2GTK's minimum size
#[cfg(target_os = "linux")]
fn force_gtk_window_size(webview_window: &tauri::WebviewWindow, width: i32, height: i32) {
    use gtk::prelude::{WidgetExt, GtkWindowExt, ContainerExt};
    use gtk::glib::object::Cast;
    if let Ok(gtk_win) = webview_window.gtk_window() {
        gtk_win.set_size_request(width, height);
        // Also force the WebView child widget to accept the small size
        for child in gtk_win.children() {
            child.set_size_request(width, height);
            // Go one level deeper — the vbox contains the webview
            if let Ok(container) = child.clone().downcast::<gtk::Container>() {
                for grandchild in container.children() {
                    grandchild.set_size_request(width, height);
                }
            }
        }
        gtk_win.set_default_size(width, height);
        gtk_win.resize(width, height);
    }
}

// === Side Button state and commands ===

struct SideButtonState {
    base_x: i32,
    base_y: i32,
    offset_x: i32,
    offset_y: i32,
    is_hidden: bool,
    animation_id: u64,
    show_completed: bool,
}

static SIDE_BUTTON_STATES: Lazy<Mutex<HashMap<String, SideButtonState>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

const SIDE_BUTTON_SLIVER: i32 = 2;
// How many pixels past the screen edge the window extends when fully shown,
// so cursor at screen edge is still inside the window (prevents jitter).
const SIDE_BUTTON_EDGE_EXTEND: i32 = 7;

#[tauri::command]
async fn read_icon_base64(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path)
        .map_err(|e| format!("Failed to read icon file '{}': {}", path, e))?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();
    let mime = match ext.as_str() {
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "image/png",
    };
    Ok(format!("data:{};base64,{}", mime, b64))
}

#[tauri::command]
async fn save_side_buttons(json: String) -> Result<(), String> {
    let home_dir = std::env::var("HOME").map_err(|_| "Failed to get HOME environment variable")?;
    let app_dir = std::path::PathBuf::from(home_dir).join(".local/share/com.bro.app");
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("Failed to create app directory: {}", e))?;
    let path = app_dir.join("side_buttons.json");
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write side buttons file: {}", e))?;
    println!("Side buttons saved to {:?}", path);
    Ok(())
}

#[tauri::command]
async fn load_side_buttons() -> Result<String, String> {
    let home_dir = std::env::var("HOME").map_err(|_| "Failed to get HOME environment variable")?;
    let path = std::path::PathBuf::from(home_dir).join(".local/share/com.bro.app/side_buttons.json");
    if path.exists() {
        let data = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read side buttons file: {}", e))?;
        Ok(data)
    } else {
        Ok("[]".to_string())
    }
}

#[tauri::command]
async fn show_side_button(
    app_handle: tauri::AppHandle,
    id: String,
    name: String,
    icon_path: String,
    command: String,
    edge: String,
    position: f64,
    last_x: Option<i32>,
    last_y: Option<i32>,
) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;
    use tauri::WebviewUrl;

    let label = format!("side-button-{}", id);

    // Close existing window if present
    if let Some(existing) = app_handle.get_webview_window(&label) {
        let _ = existing.destroy();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Determine window size and position based on edge
    // 40px icon + 4px padding each side = 48x48
    let win_w: f64 = 48.0;
    let win_h: f64 = 48.0;

    let (pos_x, pos_y) = if let (Some(lx), Some(ly)) = (last_x, last_y) {
        // Use saved position from previous drag
        (lx as f64, ly as f64)
    } else {
        // Calculate default position from edge + percentage
        let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
        let main_window = app_handle.get_webview_window("main")
            .ok_or("Main window not found")?;
        let main_pos = main_window.outer_position()
            .map_err(|e| format!("Failed to get main window position: {}", e))?;

        let current_screen = screens.iter()
            .find(|screen| {
                let d = &screen.display_info;
                main_pos.x >= d.x &&
                main_pos.x < d.x + d.width as i32 &&
                main_pos.y >= d.y &&
                main_pos.y < d.y + d.height as i32
            })
            .or_else(|| screens.first())
            .ok_or("No screen found")?;

        let d = &current_screen.display_info;
        let scale = d.scale_factor as f64;
        let screen_w = d.width as f64 / scale;
        let screen_h = d.height as f64 / scale;
        let screen_x = d.x as f64;
        let screen_y = d.y as f64;

        let ext = SIDE_BUTTON_EDGE_EXTEND as f64;
        match edge.as_str() {
            "left" => (screen_x - ext, screen_y + screen_h * position / 100.0 - win_h / 2.0),
            "right" => (screen_x + screen_w - win_w + ext, screen_y + screen_h * position / 100.0 - win_h / 2.0),
            "top" => (screen_x + screen_w * position / 100.0 - win_w / 2.0, screen_y - ext),
            "bottom" => (screen_x + screen_w * position / 100.0 - win_w / 2.0, screen_y + screen_h - win_h + ext),
            _ => (screen_x + screen_w - win_w + ext, screen_y + screen_h * position / 100.0 - win_h / 2.0),
        }
    };

    // Simple percent-encoding for URL parameters
    fn encode_uri_component(s: &str) -> String {
        let mut result = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(b as char);
                }
                _ => {
                    result.push_str(&format!("%{:02X}", b));
                }
            }
        }
        result
    }

    let encoded_icon = encode_uri_component(&icon_path);
    let encoded_cmd = encode_uri_component(&command);
    let encoded_name = encode_uri_component(&name);

    let url = format!(
        "/index.html#/side-button-overlay?id={}&iconPath={}&command={}&edge={}&name={}",
        id, encoded_icon, encoded_cmd, edge, encoded_name
    );

    let webview_window = WebviewWindowBuilder::new(
        &app_handle,
        &label,
        WebviewUrl::App(url.into())
    )
    .title(&format!("Side Button - {}", name))
    .position(pos_x, pos_y)
    .inner_size(win_w, win_h)
    .min_inner_size(win_w, win_h)
    .max_inner_size(win_w, win_h)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(true)
    .resizable(true) // keep true to avoid GTK 200px minimum floor
    .devtools(true)
    .build()
    .map_err(|e| format!("Failed to create side button window: {}", e))?;

    // Force small window size via GTK API (bypass WebKit2GTK minimum)
    // Also set type hint to Dock so WM doesn't clamp position at screen edges
    #[cfg(target_os = "linux")]
    {
        force_gtk_window_size(&webview_window, win_w as i32, win_h as i32);
        if let Ok(gtk_win) = webview_window.gtk_window() {
            use gtk::prelude::GtkWindowExt;
            gtk_win.set_type_hint(gtk::gdk::WindowTypeHint::Dock);
        }
    }

    // Wait for GTK to settle
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Insert placeholder state, then snap to correct edge position
    SIDE_BUTTON_STATES.lock().unwrap().insert(id.clone(), SideButtonState {
        base_x: 0,
        base_y: 0,
        offset_x: 0,
        offset_y: 0,
        is_hidden: false,
        animation_id: 0,
        show_completed: true,
    });

    // Snap to nearest edge — this calculates correct base, offset, and persists position
    update_side_button_base(app_handle, id).await?;

    Ok(())
}

#[tauri::command]
async fn slide_side_button_hide(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    let label = format!("side-button-{}", id);
    let window = app_handle.get_webview_window(&label)
        .ok_or("Side button window not found")?;

    // Check if cursor is actually outside the window via GDK before hiding
    let win_pos = window.outer_position()
        .map_err(|e| format!("Failed to get position: {}", e))?;
    let win_size = window.outer_size()
        .map_err(|e| format!("Failed to get size: {}", e))?;
    let wx = win_pos.x;
    let wy = win_pos.y;
    let ww = win_size.width as i32;
    let wh = win_size.height as i32;

    let (cursor_tx, cursor_rx) = std::sync::mpsc::sync_channel::<(i32, i32)>(1);
    let _ = app_handle.run_on_main_thread(move || {
        use gtk::prelude::*;
        if let Some(display) = gtk::gdk::Display::default() {
            if let Some(seat) = display.default_seat() {
                if let Some(pointer) = seat.pointer() {
                    if let Some(root) = display.default_screen().root_window() {
                        let (_win, mx, my, _mask) = root.device_position(&pointer);
                        let _ = cursor_tx.send((mx, my));
                        return;
                    }
                }
            }
        }
        let _ = cursor_tx.send((-9999, -9999));
    });

    if let Ok((mx, my)) = cursor_rx.recv() {
        // Use a margin so cursor near the edge still counts as "inside"
        let margin = 5;
        if mx >= wx - margin && mx < wx + ww + margin && my >= wy - margin && my < wy + wh + margin {
            println!("[SLIDE] HIDE id={}: SKIPPED — cursor ({},{}) is near window ({},{} {}x{})",
                     id, mx, my, wx, wy, ww, wh);
            return Ok(());
        }
        println!("[SLIDE] HIDE id={}: cursor ({},{}) is outside window ({},{} {}x{})",
                 id, mx, my, wx, wy, ww, wh);
    }

    let my_id;
    let (to_x, to_y);
    let (cur_x, cur_y);
    {
        let mut states = SIDE_BUTTON_STATES.lock().unwrap();
        let state = states.get_mut(&id).ok_or("Side button state not found")?;

        cur_x = wx;
        cur_y = wy;

        if !state.is_hidden && state.show_completed {
            println!("[SLIDE] HIDE: updating base from ({},{}) to ({},{})",
                     state.base_x, state.base_y, cur_x, cur_y);
            state.base_x = cur_x;
            state.base_y = cur_y;
        } else {
            println!("[SLIDE] HIDE: keeping base at ({},{}) (show_completed={}, is_hidden={})",
                     state.base_x, state.base_y, state.show_completed, state.is_hidden);
        }

        state.is_hidden = true;
        state.show_completed = false;
        state.animation_id += 1;
        my_id = state.animation_id;

        to_x = state.base_x + state.offset_x;
        to_y = state.base_y + state.offset_y;

        println!("[SLIDE] HIDE id={}: cur=({},{}) base=({},{}) offset=({},{}) target=({},{}) anim_id={}",
                 id, cur_x, cur_y, state.base_x, state.base_y,
                 state.offset_x, state.offset_y, to_x, to_y, my_id);
    }

    animate_side_button(&window, &id, my_id, cur_x, cur_y, to_x, to_y).await;
    Ok(())
}

#[tauri::command]
async fn slide_side_button_show(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    let label = format!("side-button-{}", id);
    let window = app_handle.get_webview_window(&label)
        .ok_or("Side button window not found")?;

    let my_id;
    let (to_x, to_y);
    let (cur_x, cur_y);
    {
        let mut states = SIDE_BUTTON_STATES.lock().unwrap();
        let state = states.get_mut(&id).ok_or("Side button state not found")?;

        // Read actual current position (may be mid-animation)
        let pos = window.outer_position()
            .map_err(|e| format!("Failed to get position: {}", e))?;
        cur_x = pos.x;
        cur_y = pos.y;

        state.is_hidden = false;
        state.show_completed = false; // Will be set to true when animation completes
        state.animation_id += 1;
        my_id = state.animation_id;

        // Always animate to base
        to_x = state.base_x;
        to_y = state.base_y;

        println!("[SLIDE] SHOW id={}: cur=({},{}) base=({},{}) target=({},{}) anim_id={}",
                 id, cur_x, cur_y, state.base_x, state.base_y, to_x, to_y, my_id);
    }

    let completed = animate_side_button(&window, &id, my_id, cur_x, cur_y, to_x, to_y).await;

    // Mark show as completed only if animation wasn't cancelled
    if completed {
        let mut states = SIDE_BUTTON_STATES.lock().unwrap();
        if let Some(state) = states.get_mut(&id) {
            if state.animation_id == my_id {
                state.show_completed = true;
                println!("[SLIDE] SHOW id={}: show_completed=true", id);
            }
        }
    }

    Ok(())
}

async fn animate_side_button(
    window: &tauri::WebviewWindow,
    id: &str,
    my_id: u64,
    from_x: i32, from_y: i32,
    to_x: i32, to_y: i32,
) -> bool {
    // Skip animation if already at target
    if from_x == to_x && from_y == to_y {
        println!("[SLIDE] ANIMATE id={} anim_id={}: already at target ({},{}), skipping",
                 id, my_id, to_x, to_y);
        return true;
    }

    let steps = 10;
    println!("[SLIDE] ANIMATE id={} anim_id={}: ({},{}) -> ({},{})",
             id, my_id, from_x, from_y, to_x, to_y);

    for i in 1..=steps {
        // Check if animation was cancelled
        {
            let states = SIDE_BUTTON_STATES.lock().unwrap();
            if let Some(state) = states.get(id) {
                if state.animation_id != my_id {
                    println!("[SLIDE] ANIMATE id={} anim_id={}: CANCELLED at step {}/{}",
                             id, my_id, i, steps);
                    return false;
                }
            } else { return false; }
        }

        let t = i as f64 / steps as f64;
        let ease = 1.0 - (1.0 - t).powi(3);
        let x = from_x as f64 + (to_x - from_x) as f64 * ease;
        let y = from_y as f64 + (to_y - from_y) as f64 * ease;

        let result = window.set_position(tauri::Position::Physical(
            tauri::PhysicalPosition::new(x as i32, y as i32)
        ));

        if let Err(e) = result {
            println!("[SLIDE] ANIMATE id={}: set_position FAILED at step {}: {}", id, i, e);
            return false;
        }

        if i < steps {
            tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
        }
    }
    println!("[SLIDE] ANIMATE id={} anim_id={}: DONE at ({},{})", id, my_id, to_x, to_y);
    true
}

#[tauri::command]
async fn hide_side_button(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    let label = format!("side-button-{}", id);
    SIDE_BUTTON_STATES.lock().unwrap().remove(&id);
    if let Some(window) = app_handle.get_webview_window(&label) {
        window.destroy().map_err(|e| format!("Failed to destroy side button window: {}", e))?;
        println!("Side button '{}' destroyed", id);
    }
    Ok(())
}

#[tauri::command]
async fn launch_side_button_app(command: String) -> Result<(), String> {
    println!("Launching side button app: {}", command);

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    use std::os::unix::process::CommandExt;

    let mut cmd = std::process::Command::new(parts[0]);
    if parts.len() > 1 {
        cmd.args(&parts[1..]);
    }

    // Detach: new session so child survives parent exit
    unsafe {
        cmd.pre_exec(|| {
            extern "C" { fn setsid() -> i32; }
            setsid();
            Ok(())
        });
    }

    cmd.stdin(std::process::Stdio::null())
       .stdout(std::process::Stdio::null())
       .stderr(std::process::Stdio::null());

    cmd.spawn()
       .map_err(|e| format!("Failed to launch '{}': {}", command, e))?;

    Ok(())
}

#[tauri::command]
async fn start_side_button_drag(
    app_handle: tauri::AppHandle,
    id: String,
    start_root_x: i32,
    start_root_y: i32,
) -> Result<(), String> {
    let label = format!("side-button-{}", id);
    let window = app_handle.get_webview_window(&label)
        .ok_or("Side button window not found")?;

    let start_pos = window.outer_position()
        .map_err(|e| format!("Failed to get position: {}", e))?;
    let win_start_x = start_pos.x;
    let win_start_y = start_pos.y;

    let (tx, rx) = std::sync::mpsc::sync_channel::<()>(1);
    let app_clone = app_handle.clone();
    let label_clone = label.clone();
    let label_for_grab = label.clone();
    let app_for_grab = app_handle.clone();

    app_handle.run_on_main_thread(move || {
        use std::rc::Rc;
        use std::cell::RefCell;
        use gtk::prelude::*;

        let tx = Rc::new(RefCell::new(Some(tx)));

        let display = match gtk::gdk::Display::default() {
            Some(d) => d,
            None => { if let Some(tx) = tx.borrow_mut().take() { let _ = tx.send(()); } return; }
        };
        let seat = match display.default_seat() {
            Some(s) => s,
            None => { if let Some(tx) = tx.borrow_mut().take() { let _ = tx.send(()); } return; }
        };
        let pointer = match seat.pointer() {
            Some(p) => p,
            None => { if let Some(tx) = tx.borrow_mut().take() { let _ = tx.send(()); } return; }
        };
        let root_window = match display.default_screen().root_window() {
            Some(w) => w,
            None => { if let Some(tx) = tx.borrow_mut().take() { let _ = tx.send(()); } return; }
        };

        // Grab the pointer on the button's own GDK window (prevents text selection in other apps)
        if let Some(win) = app_for_grab.get_webview_window(&label_for_grab) {
            if let Ok(gtk_win) = win.gtk_window() {
                if let Some(gdk_win) = gtk_win.window() {
                    let _ = seat.grab(
                        &gdk_win,
                        gtk::gdk::SeatCapabilities::POINTER,
                        true,  // owner_events
                        None,  // cursor
                        None,  // event
                        None::<&mut dyn FnMut(&gtk::gdk::Seat, &gtk::gdk::Window)>,
                    );
                }
            }
        }
        let seat_for_ungrab = seat.clone();

        gtk::glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let (_win, mouse_x, mouse_y, mask) = root_window.device_position(&pointer);

            if !mask.contains(gtk::gdk::ModifierType::BUTTON1_MASK) {
                seat_for_ungrab.ungrab();
                if let Some(tx) = tx.borrow_mut().take() { let _ = tx.send(()); }
                return gtk::glib::ControlFlow::Break;
            }

            let new_x = win_start_x + (mouse_x - start_root_x);
            let new_y = win_start_y + (mouse_y - start_root_y);

            if let Some(win) = app_clone.get_webview_window(&label_clone) {
                let _ = win.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition::new(new_x, new_y)
                ));
            }

            gtk::glib::ControlFlow::Continue
        });
    }).map_err(|e| format!("Main thread error: {:?}", e))?;

    // Wait for drag to complete (blocks this async task, not the main thread)
    let _ = rx.recv();

    // Snap to nearest edge
    update_side_button_base(app_handle, id).await?;

    Ok(())
}

#[tauri::command]
async fn update_side_button_base(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    let label = format!("side-button-{}", id);
    let window = app_handle.get_webview_window(&label)
        .ok_or("Side button window not found")?;

    let pos = window.outer_position()
        .map_err(|e| format!("Failed to get position: {}", e))?;
    let size = window.outer_size()
        .map_err(|e| format!("Failed to get size: {}", e))?;

    let wx = pos.x;
    let wy = pos.y;
    let ww = size.width as i32;
    let wh = size.height as i32;

    // Find which screen the window center is on
    let cx = wx + ww / 2;
    let cy = wy + wh / 2;

    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let screen = screens.iter()
        .find(|s| {
            let d = &s.display_info;
            cx >= d.x && cx < d.x + d.width as i32 &&
            cy >= d.y && cy < d.y + d.height as i32
        })
        .or_else(|| screens.first())
        .ok_or("No screen found")?;

    let d = &screen.display_info;
    let sx = d.x;
    let sy = d.y;
    let sw = d.width as i32;
    let sh = d.height as i32;

    // Calculate distances to each edge
    let dist_left = (wx - sx).abs();
    let dist_right = ((sx + sw) - (wx + ww)).abs();
    let dist_top = (wy - sy).abs();
    let dist_bottom = ((sy + sh) - (wy + wh)).abs();

    let min_dist = dist_left.min(dist_right).min(dist_top).min(dist_bottom);

    let ext = SIDE_BUTTON_EDGE_EXTEND;
    let (snap_x, snap_y, off_x, off_y) = if min_dist == dist_right {
        (sx + sw - ww + ext, wy, ww - SIDE_BUTTON_SLIVER - ext, 0)
    } else if min_dist == dist_left {
        (sx - ext, wy, -(ww - SIDE_BUTTON_SLIVER - ext), 0)
    } else if min_dist == dist_bottom {
        (wx, sy + sh - wh + ext, 0, wh - SIDE_BUTTON_SLIVER - ext)
    } else {
        (wx, sy - ext, 0, -(wh - SIDE_BUTTON_SLIVER - ext))
    };

    // Snap window to nearest edge
    let _ = window.set_position(tauri::Position::Physical(
        tauri::PhysicalPosition::new(snap_x, snap_y)
    ));

    let mut states = SIDE_BUTTON_STATES.lock().unwrap();
    if let Some(state) = states.get_mut(&id) {
        println!("[SLIDE] SNAP to edge: id={} drag=({},{}) snap=({},{}) offset=({},{})",
                 id, wx, wy, snap_x, snap_y, off_x, off_y);
        state.base_x = snap_x;
        state.base_y = snap_y;
        state.offset_x = off_x;
        state.offset_y = off_y;
        state.show_completed = true;
        state.is_hidden = false;
    }
    drop(states);

    // Set flipped cursor for right-edge buttons
    let needs_hflip = min_dist == dist_right;
    let app_for_cursor = app_handle.clone();
    let label_for_cursor = label.clone();
    let _ = app_handle.run_on_main_thread(move || {
        use gtk::prelude::*;

        let display = match gtk::gdk::Display::default() {
            Some(d) => d,
            None => return,
        };

        let gdk_win = match app_for_cursor.get_webview_window(&label_for_cursor)
            .and_then(|w| w.gtk_window().ok())
            .and_then(|gtk_win| gtk_win.window())
        {
            Some(w) => w,
            None => return,
        };

        if !needs_hflip {
            gdk_win.set_cursor(None::<&gtk::gdk::Cursor>);
            return;
        }

        // Get the default cursor and extract its image
        let cursor = match gtk::gdk::Cursor::from_name(&display, "default") {
            Some(c) => c,
            None => return,
        };
        let pixbuf = match cursor.image() {
            Some(p) => p,
            None => return,
        };

        // Read hotspot from pixbuf metadata
        let hot_x: i32 = pixbuf.option("x_hot")
            .and_then(|v| v.parse().ok()).unwrap_or(0);
        let hot_y: i32 = pixbuf.option("y_hot")
            .and_then(|v| v.parse().ok()).unwrap_or(0);

        // Flip horizontally
        let flipped = match pixbuf.flip(true) {
            Some(f) => f,
            None => return,
        };

        // Mirror the hotspot x coordinate
        let new_hot_x = flipped.width() - 1 - hot_x;
        let flipped_cursor = gtk::gdk::Cursor::from_pixbuf(
            &display, &flipped, new_hot_x, hot_y,
        );
        gdk_win.set_cursor(Some(&flipped_cursor));
    });

    // Persist snapped position to side_buttons.json
    let home_dir = std::env::var("HOME").unwrap_or_default();
    let path = std::path::PathBuf::from(&home_dir).join(".local/share/com.bro.app/side_buttons.json");
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(mut arr) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                for item in arr.iter_mut() {
                    if item.get("id").and_then(|v| v.as_str()) == Some(&id) {
                        item["lastX"] = serde_json::json!(snap_x);
                        item["lastY"] = serde_json::json!(snap_y);
                        break;
                    }
                }
                if let Ok(json) = serde_json::to_string(&arr) {
                    let _ = std::fs::write(&path, json);
                }
            }
        }
    }

    Ok(())
}

// Команда для выполнения действий кнопки
#[tauri::command]
async fn execute_button_actions(template_id: String) -> Result<(), String> {
    use serde::{Deserialize, Serialize};
    use enigo::{Enigo, Mouse, Settings, Keyboard, Key};

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "type")]
    enum Action {
        #[serde(rename = "click")]
        Click { x: i32, y: i32, button: String },
        #[serde(rename = "keypress")]
        Keypress { keys: String },
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct ButtonTemplate {
        id: String,
        name: String,
        actions: Vec<Action>,
    }

    println!("Executing actions for template: {}", template_id);

    // Читаем шаблоны из localStorage (через файл)
    let home_dir = std::env::var("HOME").map_err(|_| "Failed to get HOME environment variable")?;
    let local_storage_path = std::path::PathBuf::from(home_dir).join(".local/share/com.bro.app/button_templates.json");

    // Если файл не существует, пытаемся прочитать из browser localStorage
    let templates: Vec<ButtonTemplate> = if local_storage_path.exists() {
        let data = std::fs::read_to_string(&local_storage_path)
            .map_err(|e| format!("Failed to read templates file: {}", e))?;
        serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse templates: {}", e))?
    } else {
        println!("Templates file not found, returning error");
        return Err("No templates found".to_string());
    };

    // Находим нужный шаблон
    let template = templates.iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| format!("Template {} not found", template_id))?;

    println!("Found template '{}' with {} actions", template.name, template.actions.len());

    // Выполняем действия
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Failed to create Enigo: {}", e))?;

    for (i, action) in template.actions.iter().enumerate() {
        println!("Executing action {}/{}", i + 1, template.actions.len());

        match action {
            Action::Click { x, y, button } => {
                println!("Clicking at ({}, {}) with {} button", x, y, button);

                // Перемещаем курсор
                let _ = enigo.move_mouse(*x, *y, enigo::Coordinate::Abs);
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                // Выполняем клик
                match button.as_str() {
                    "left" => {
                        let _ = enigo.button(enigo::Button::Left, enigo::Direction::Press);
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        let _ = enigo.button(enigo::Button::Left, enigo::Direction::Release);
                    }
                    "right" => {
                        let _ = enigo.button(enigo::Button::Right, enigo::Direction::Press);
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        let _ = enigo.button(enigo::Button::Right, enigo::Direction::Release);
                    }
                    _ => println!("Unknown button type: {}", button),
                }
            }
            Action::Keypress { keys } => {
                println!("Pressing keys: {}", keys);

                // Парсим клавиши из строки типа "Ctrl+C"
                let key_parts: Vec<&str> = keys.split('+').collect();
                let mut modifiers = Vec::new();
                let mut main_key = None;

                for part in key_parts.iter() {
                    match part.trim().to_lowercase().as_str() {
                        "ctrl" | "control" => modifiers.push(Key::Control),
                        "alt" => modifiers.push(Key::Alt),
                        "shift" => modifiers.push(Key::Shift),
                        "super" | "meta" => modifiers.push(Key::Meta),
                        key_str => {
                            // Основная клавиша
                            main_key = Some(key_str.to_string());
                        }
                    }
                }

                // Нажимаем модификаторы
                for modifier in &modifiers {
                    let _ = enigo.key(*modifier, enigo::Direction::Press);
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }

                // Нажимаем основную клавишу
                if let Some(key_str) = main_key {
                    // Преобразуем строку в клавишу
                    let key = match key_str.to_uppercase().as_str() {
                        "A" => Key::Unicode('a'),
                        "B" => Key::Unicode('b'),
                        "C" => Key::Unicode('c'),
                        "D" => Key::Unicode('d'),
                        "E" => Key::Unicode('e'),
                        "F" => Key::Unicode('f'),
                        "G" => Key::Unicode('g'),
                        "H" => Key::Unicode('h'),
                        "I" => Key::Unicode('i'),
                        "J" => Key::Unicode('j'),
                        "K" => Key::Unicode('k'),
                        "L" => Key::Unicode('l'),
                        "M" => Key::Unicode('m'),
                        "N" => Key::Unicode('n'),
                        "O" => Key::Unicode('o'),
                        "P" => Key::Unicode('p'),
                        "Q" => Key::Unicode('q'),
                        "R" => Key::Unicode('r'),
                        "S" => Key::Unicode('s'),
                        "T" => Key::Unicode('t'),
                        "U" => Key::Unicode('u'),
                        "V" => Key::Unicode('v'),
                        "W" => Key::Unicode('w'),
                        "X" => Key::Unicode('x'),
                        "Y" => Key::Unicode('y'),
                        "Z" => Key::Unicode('z'),
                        "ENTER" | "RETURN" => Key::Return,
                        "TAB" => Key::Tab,
                        "SPACE" => Key::Space,
                        "ESCAPE" | "ESC" => Key::Escape,
                        "F1" => Key::F1,
                        "F2" => Key::F2,
                        "F3" => Key::F3,
                        "F4" => Key::F4,
                        "F5" => Key::F5,
                        "F6" => Key::F6,
                        "F7" => Key::F7,
                        "F8" => Key::F8,
                        "F9" => Key::F9,
                        "F10" => Key::F10,
                        "F11" => Key::F11,
                        "F12" => Key::F12,
                        "DELETE" | "DEL" => Key::Delete,
                        "BACKSPACE" => Key::Backspace,
                        "HOME" => Key::Home,
                        "END" => Key::End,
                        "PAGEUP" | "PAGE UP" => Key::PageUp,
                        "PAGEDOWN" | "PAGE DOWN" => Key::PageDown,
                        "INSERT" | "INS" => Key::Insert,
                        "ARROWUP" | "UP" => Key::UpArrow,
                        "ARROWDOWN" | "DOWN" => Key::DownArrow,
                        "ARROWLEFT" | "LEFT" => Key::LeftArrow,
                        "ARROWRIGHT" | "RIGHT" => Key::RightArrow,
                        _ => Key::Unicode(key_str.chars().next().unwrap_or('a')),
                    };

                    let _ = enigo.key(key, enigo::Direction::Click);
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }

                // Отпускаем модификаторы
                for modifier in modifiers.iter().rev() {
                    let _ = enigo.key(*modifier, enigo::Direction::Release);
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            }
        }

        // Небольшая задержка между действиями
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("All actions executed successfully");
    Ok(())
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
        .manage(StreamingServer::new())
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

            // Регистрируем Super+PrintScreen для скриншота с задержкой 5 секунд
            match app.global_shortcut().on_shortcut("Super+PrintScreen", move |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    println!("Super+PrintScreen pressed - delayed screenshot capture");
                    if let Some(window) = app.get_webview_window("main") {
                        // НЕ переключаем фокус на окно, отправляем событие в фоне
                        let _ = window.eval("window.dispatchEvent(new CustomEvent('delayed-screenshot-hotkey-pressed'))");
                    }
                }
            }) {
                Ok(_) => println!("Super+PrintScreen shortcut registered successfully"),
                Err(e) => eprintln!("Failed to register Super+PrintScreen shortcut: {}", e),
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

            // Enable media permissions for WebView on Linux
            #[cfg(target_os = "linux")]
            {
                use webkit2gtk::{WebViewExt, SettingsExt, PermissionRequestExt};
                use webkit2gtk::glib::Cast;

                if let Some(window) = app.get_webview_window("main") {
                    if let Ok(_) = window.with_webview(|webview| {
                        use webkit2gtk::WebView;
                        if let Ok(wv) = webview.inner().clone().downcast::<WebView>() {
                            // Enable media settings
                            if let Some(settings) = wv.settings() {
                                settings.set_enable_media_stream(true);
                                settings.set_enable_mediasource(true);
                                settings.set_enable_media(true);
                                settings.set_enable_media_capabilities(true);
                                println!("WebView media settings enabled");
                            }

                            // Handle permission requests - automatically allow all
                            wv.connect_permission_request(|_webview, request| {
                                println!("Permission request received: {:?}", request);
                                request.allow();
                                true // Stop propagation
                            });

                            println!("Permission handler installed");
                        }
                    }) {
                        println!("WebView configuration completed");
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_cpu_temperature,
            get_network_speed,
            capture_full_screenshot,
            capture_monitor_screenshot,
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
            perform_click,
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
            play_click_sequence,
            get_local_ip,
            start_screen_streaming,
            stop_screen_streaming,
            execute_command_stream,
            find_image_on_screen,
            save_button_templates,
            load_button_templates,
            show_overlay_button,
            hide_overlay_button,
            execute_button_actions,
            read_icon_base64,
            save_side_buttons,
            load_side_buttons,
            show_side_button,
            hide_side_button,
            slide_side_button_hide,
            slide_side_button_show,
            launch_side_button_app,
            start_side_button_drag,
            update_side_button_base,
            create_pty_session,
            write_to_pty,
            resize_pty,
            kill_pty_session,
            get_pty_sessions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

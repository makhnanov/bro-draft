use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tungstenite::{accept, Message};
use screenshots::Screen;
use std::io::{Cursor, Write};
use image::ImageEncoder;

pub fn start_websocket_server(
    port: u16,
    screen_index: usize,
    stop_signal: Arc<AtomicBool>
) -> Result<std::thread::JoinHandle<()>, String> {
    let addr = format!("0.0.0.0:{}", port);
    println!("Starting hybrid HTTP/WebSocket server on {}", addr);

    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("Failed to bind server: {}", e))?;

    listener.set_nonblocking(true)
        .map_err(|e| format!("Failed to set nonblocking: {}", e))?;

    println!("Hybrid server listening on {}", addr);

    let handle = thread::spawn(move || {
        for stream in listener.incoming() {
            if stop_signal.load(Ordering::SeqCst) {
                println!("Stop signal received, shutting down server");
                break;
            }

            match stream {
                Ok(stream) => {
                    let stop_signal_clone = stop_signal.clone();
                    thread::spawn(move || {
                        if let Err(e) = handle_connection(stream, stop_signal_clone, screen_index) {
                            eprintln!("Client error: {}", e);
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                }
            }
        }
        println!("Server stopped");
    });

    Ok(handle)
}

fn handle_connection(
    stream: TcpStream,
    stop_signal: Arc<AtomicBool>,
    screen_index: usize
) -> Result<(), String> {
    // Читаем начало запроса чтобы определить тип (HTTP или WebSocket)
    let mut buffer = [0u8; 8192];

    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

    // Читаем данные с помощью peek (не удаляем из буфера)
    let n = match stream.peek(&mut buffer) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to peek stream: {}", e);
            return Err(e.to_string());
        }
    };

    if n == 0 {
        return Err("Empty request".to_string());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
    let first_line = request.lines().next().unwrap_or("");

    println!("Request: {}", first_line);

    // Проверяем наличие WebSocket upgrade
    let is_websocket = request.to_lowercase().contains("upgrade: websocket") ||
                      request.to_lowercase().contains("upgrade:websocket");

    if is_websocket {
        println!("WebSocket connection detected");
        handle_websocket(stream, stop_signal, screen_index)
    } else {
        println!("HTTP connection detected");
        handle_http(stream)
    }
}

fn handle_http(mut stream: TcpStream) -> Result<(), String> {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>WebSocket Screen Stream</title>
    <style>
        body {
            margin: 0;
            padding: 20px;
            background: #1a1a1a;
            color: #fff;
            font-family: Arial, sans-serif;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
        }
        h1 {
            margin-bottom: 20px;
        }
        canvas {
            max-width: 90vw;
            max-height: 80vh;
            border: 2px solid #333;
            border-radius: 8px;
            background: #000;
        }
        .info {
            margin-top: 20px;
            padding: 15px;
            background: #2a2a2a;
            border-radius: 8px;
            text-align: center;
        }
        .status {
            color: #4caf50;
            font-weight: bold;
        }
        .error {
            color: #f44336;
        }
    </style>
</head>
<body>
    <h1>WebSocket Screen Stream</h1>
    <canvas id="stream"></canvas>
    <div class="info">
        <div id="status" class="status">Connecting...</div>
        <p id="fps">FPS: 0 | Latency: 0ms</p>
    </div>
    <script>
        const canvas = document.getElementById('stream');
        const ctx = canvas.getContext('2d');
        const statusDiv = document.getElementById('status');
        const fpsDisplay = document.getElementById('fps');

        let frameCount = 0;
        let fps = 0;
        let lastUpdate = Date.now();
        let lastFrameTime = Date.now();

        const ws = new WebSocket('ws://' + window.location.host);
        ws.binaryType = 'arraybuffer';

        ws.onopen = function() {
            console.log('WebSocket connected');
            statusDiv.textContent = '● Live';
            statusDiv.className = 'status';
        };

        ws.onmessage = function(event) {
            const now = Date.now();
            const latency = now - lastFrameTime;
            lastFrameTime = now;

            const blob = new Blob([event.data], { type: 'image/jpeg' });
            const url = URL.createObjectURL(blob);

            const img = new Image();
            img.onload = function() {
                if (canvas.width === 0) {
                    canvas.width = img.width;
                    canvas.height = img.height;
                }

                ctx.drawImage(img, 0, 0);
                URL.revokeObjectURL(url);

                frameCount++;
                if (now - lastUpdate >= 1000) {
                    fps = frameCount;
                    fpsDisplay.textContent = `FPS: ${fps} | Latency: ${latency}ms`;
                    frameCount = 0;
                    lastUpdate = now;
                }
            };
            img.src = url;
        };

        ws.onerror = function(error) {
            console.error('WebSocket error:', error);
            statusDiv.textContent = '● Error';
            statusDiv.className = 'error';
        };

        ws.onclose = function() {
            console.log('WebSocket disconnected');
            statusDiv.textContent = '● Disconnected';
            statusDiv.className = 'error';
        };
    </script>
</body>
</html>"#;

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );

    stream.write_all(response.as_bytes()).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn handle_websocket(
    stream: TcpStream,
    stop_signal: Arc<AtomicBool>,
    screen_index: usize
) -> Result<(), String> {
    let mut websocket = accept(stream)
        .map_err(|e| format!("WebSocket handshake failed: {}", e))?;

    println!("WebSocket handshake successful, starting stream...");

    let mut frame_count = 0u64;
    loop {
        if stop_signal.load(Ordering::SeqCst) {
            println!("Stop signal received, closing client");
            break;
        }

        let screens = Screen::all()
            .map_err(|e| format!("Failed to get screens: {}", e))?;

        let screen = screens.get(screen_index)
            .ok_or_else(|| format!("Screen {} not found", screen_index))?;

        let captured = screen.capture()
            .map_err(|e| format!("Failed to capture: {}", e))?;

        let width = captured.width();
        let height = captured.height();
        let rgba_data = captured.rgba();

        let pixel_count = (width * height) as usize;
        let mut rgb_data = Vec::with_capacity(pixel_count * 3);
        unsafe {
            rgb_data.set_len(pixel_count * 3);
        }

        let mut rgb_idx = 0;
        for rgba_idx in (0..pixel_count * 4).step_by(4) {
            rgb_data[rgb_idx] = rgba_data[rgba_idx];
            rgb_data[rgb_idx + 1] = rgba_data[rgba_idx + 1];
            rgb_data[rgb_idx + 2] = rgba_data[rgba_idx + 2];
            rgb_idx += 3;
        }

        let mut jpeg_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut jpeg_data);
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 60);
            if let Err(e) = encoder.write_image(&rgb_data, width, height, image::ExtendedColorType::Rgb8) {
                eprintln!("Failed to encode JPEG: {}", e);
                continue;
            }
        }

        if let Err(e) = websocket.send(Message::Binary(jpeg_data)) {
            eprintln!("Failed to send frame: {}", e);
            break;
        }

        frame_count += 1;
        if frame_count % 30 == 0 {
            println!("Sent {} frames", frame_count);
        }

        thread::sleep(Duration::from_millis(50));
    }

    println!("Client disconnected, sent {} frames total", frame_count);
    Ok(())
}

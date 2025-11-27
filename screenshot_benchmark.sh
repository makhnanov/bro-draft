#!/bin/bash

# Screenshot FPS Benchmark Script
# Creates screenshots for 10 seconds and calculates FPS

# Устанавливаем локаль для правильной работы с числами
export LC_NUMERIC=C

echo "=== Screenshot FPS Benchmark ==="
echo ""

# Получаем список мониторов
echo "Available monitors:"
xrandr --listmonitors | grep -E '^ [0-9]+:'
echo ""

# Спрашиваем какой монитор использовать
read -p "Enter monitor number (0, 1, etc.) or 'a' for all: " MONITOR_CHOICE
echo ""

# Создаём директорию для скриншотов
SCREENSHOT_DIR="/var/www/screenshots_temp"
rm -rf "$SCREENSHOT_DIR" 2>/dev/null
mkdir -p "$SCREENSHOT_DIR"

echo "Screenshot directory: $SCREENSHOT_DIR"
echo "Duration: 10 seconds"

if [ "$MONITOR_CHOICE" == "a" ] || [ "$MONITOR_CHOICE" == "A" ]; then
    echo "Capturing: ALL monitors"
    echo "Using tool: scrot"
    CAPTURE_METHOD="scrot"
    GEOMETRY=""
else
    echo "Capturing: Monitor $MONITOR_CHOICE"
    echo "Using tool: import (ImageMagick)"
    CAPTURE_METHOD="import"

    # Получаем геометрию монитора (формат: 1920/521x1080/293+1920+0)
    MONITOR_INFO=$(xrandr --listmonitors | grep "^ $MONITOR_CHOICE:" | awk '{print $3}')
    if [ -z "$MONITOR_INFO" ]; then
        echo "Error: Monitor $MONITOR_CHOICE not found!"
        exit 1
    fi

    # Парсим геометрию: убираем физические размеры, оставляем ширина x высота +x +y
    # Из "1920/521x1080/293+1920+0" делаем "1920x1080+1920+0"
    WIDTH=$(echo $MONITOR_INFO | cut -d'/' -f1)
    REST=$(echo $MONITOR_INFO | cut -d'x' -f2)
    HEIGHT=$(echo $REST | cut -d'/' -f1)
    OFFSET=$(echo $REST | cut -d'+' -f2-)
    GEOMETRY="${WIDTH}x${HEIGHT}+${OFFSET}"

    echo "Geometry: $GEOMETRY"
fi

echo ""

# Счётчик кадров
FRAME_COUNT=0
START_TIME=$(date +%s.%N)
END_TIME=$(echo "$START_TIME + 10" | bc)

echo "Starting benchmark..."
echo ""

# Цикл захвата скриншотов
while true; do
    CURRENT_TIME=$(date +%s.%N)

    # Проверяем прошло ли 10 секунд
    if (( $(echo "$CURRENT_TIME >= $END_TIME" | bc -l) )); then
        break
    fi

    # Захватываем скриншот
    if [ "$CAPTURE_METHOD" == "scrot" ]; then
        # Все мониторы - используем scrot
        scrot -q 100 "$SCREENSHOT_DIR/screenshot_$FRAME_COUNT.png" 2>/dev/null
    else
        # Конкретный монитор - используем import с crop
        import -window root -crop "$GEOMETRY" "$SCREENSHOT_DIR/screenshot_$FRAME_COUNT.png" 2>/dev/null
    fi

    FRAME_COUNT=$((FRAME_COUNT + 1))

    # Показываем прогресс каждые 10 кадров
    if [ $((FRAME_COUNT % 10)) -eq 0 ]; then
        ELAPSED=$(echo "$CURRENT_TIME - $START_TIME" | bc)
        CURRENT_FPS=$(echo "scale=2; $FRAME_COUNT / $ELAPSED" | bc)
        printf "\rFrames: %d | Elapsed: %.1fs | Current FPS: %.2f  " $FRAME_COUNT $ELAPSED $CURRENT_FPS
    fi
done

# Финальный расчёт
ACTUAL_END_TIME=$(date +%s.%N)
DURATION=$(echo "$ACTUAL_END_TIME - $START_TIME" | bc)
FPS=$(echo "scale=2; $FRAME_COUNT / $DURATION" | bc)

# Размер файлов
TOTAL_SIZE=$(du -sh "$SCREENSHOT_DIR" 2>/dev/null | cut -f1)
FILE_COUNT=$(ls -1 "$SCREENSHOT_DIR" 2>/dev/null | wc -l)

echo -e "\n"
echo "=== Results ==="
echo "Duration: $DURATION seconds"
echo "Total frames captured: $FRAME_COUNT"
echo "Files created: $FILE_COUNT"
echo "Average FPS: $FPS"
echo "Total size: $TOTAL_SIZE"
echo ""
echo "Screenshots saved in: $SCREENSHOT_DIR"
echo ""

# Спрашиваем удалить ли файлы
read -p "Delete screenshots? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf "$SCREENSHOT_DIR"
    echo "Screenshots deleted"
else
    echo "Screenshots kept in: $SCREENSHOT_DIR"
fi

#!/bin/bash

# Continuous High-Quality Screenshot Capture to Shared Folder
# Captures screenshots continuously and saves to shared directory

export LC_NUMERIC=C

SHARE_DIR="/home/romaxa/share"

# Создаём директорию если её нет
mkdir -p "$SHARE_DIR"

echo "=== Continuous Screenshot Capture ==="
echo ""
echo "Share directory: $SHARE_DIR"
echo ""

# Показываем доступные мониторы
echo "Available monitors:"
xrandr --listmonitors | grep -E '^ [0-9]+:'
echo ""

# Спрашиваем какой монитор
read -p "Enter monitor number (0, 1, etc.) or 'a' for all: " MONITOR_CHOICE
echo ""

# Спрашиваем интервал
read -p "Enter capture interval in seconds (default: 1): " INTERVAL
INTERVAL=${INTERVAL:-1}
echo ""

echo "Starting continuous capture..."
echo "Monitor: $MONITOR_CHOICE"
echo "Interval: ${INTERVAL}s"
echo "Press Ctrl+C to stop"
echo ""

FRAME_COUNT=0
START_TIME=$(date +%s)

if [ "$MONITOR_CHOICE" == "a" ] || [ "$MONITOR_CHOICE" == "A" ]; then
    # Все мониторы
    echo "Capturing all monitors continuously..."
    echo ""

    while true; do
        TIMESTAMP=$(date +"%Y%m%d_%H%M%S_%N")
        OUTPUT_FILE="${SHARE_DIR}/screenshot_all_${TIMESTAMP}.png"

        scrot -q 100 "$OUTPUT_FILE" 2>/dev/null

        if [ -f "$OUTPUT_FILE" ]; then
            # Создаём симлинк на последний скриншот
            ln -sf "$OUTPUT_FILE" "${SHARE_DIR}/latest_screenshot.png"

            FRAME_COUNT=$((FRAME_COUNT + 1))
            ELAPSED=$(($(date +%s) - START_TIME))

            if [ $ELAPSED -gt 0 ]; then
                AVG_FPS=$(echo "scale=2; $FRAME_COUNT / $ELAPSED" | bc)
            else
                AVG_FPS="0"
            fi

            SIZE=$(du -h "$OUTPUT_FILE" | cut -f1)

            printf "\r[%s] Frame: %d | Time: %ds | Avg FPS: %s | Size: %s  " \
                "$(date +%H:%M:%S)" $FRAME_COUNT $ELAPSED $AVG_FPS "$SIZE"
        else
            echo "Failed to capture frame $FRAME_COUNT"
        fi

        sleep $INTERVAL
    done
else
    # Конкретный монитор
    # Получаем геометрию монитора
    MONITOR_INFO=$(xrandr --listmonitors | grep "^ $MONITOR_CHOICE:" | awk '{print $3}')
    if [ -z "$MONITOR_INFO" ]; then
        echo "Error: Monitor $MONITOR_CHOICE not found!"
        exit 1
    fi

    # Парсим геометрию
    WIDTH=$(echo $MONITOR_INFO | cut -d'/' -f1)
    REST=$(echo $MONITOR_INFO | cut -d'x' -f2)
    HEIGHT=$(echo $REST | cut -d'/' -f1)
    OFFSET=$(echo $REST | cut -d'+' -f2-)
    GEOMETRY="${WIDTH}x${HEIGHT}+${OFFSET}"

    echo "Capturing monitor $MONITOR_CHOICE ($GEOMETRY) continuously..."
    echo ""

    while true; do
        TIMESTAMP=$(date +"%Y%m%d_%H%M%S_%N")
        OUTPUT_FILE="${SHARE_DIR}/screenshot_monitor${MONITOR_CHOICE}_${TIMESTAMP}.png"

        import -window root -crop "$GEOMETRY" "$OUTPUT_FILE" 2>/dev/null

        if [ -f "$OUTPUT_FILE" ]; then
            # Создаём симлинк на последний скриншот
            ln -sf "$OUTPUT_FILE" "${SHARE_DIR}/latest_screenshot.png"

            FRAME_COUNT=$((FRAME_COUNT + 1))
            ELAPSED=$(($(date +%s) - START_TIME))

            if [ $ELAPSED -gt 0 ]; then
                AVG_FPS=$(echo "scale=2; $FRAME_COUNT / $ELAPSED" | bc)
            else
                AVG_FPS="0"
            fi

            SIZE=$(du -h "$OUTPUT_FILE" | cut -f1)

            printf "\r[%s] Frame: %d | Time: %ds | Avg FPS: %s | Size: %s  " \
                "$(date +%H:%M:%S)" $FRAME_COUNT $ELAPSED $AVG_FPS "$SIZE"
        else
            echo "Failed to capture frame $FRAME_COUNT"
        fi

        sleep $INTERVAL
    done
fi

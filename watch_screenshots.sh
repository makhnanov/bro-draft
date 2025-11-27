#!/bin/bash

# Screenshot Auto-Viewer with Network Support
# Monitors shared folder (local or network) and automatically displays new screenshots

echo "=== Screenshot Auto-Viewer ==="
echo ""
echo "This script can monitor screenshots from:"
echo "1. Local directory (same computer)"
echo "2. Network share (Samba/CIFS)"
echo "3. SSH/SFTP mount"
echo ""

read -p "Enter option (1/2/3): " SOURCE_TYPE
echo ""

if [ "$SOURCE_TYPE" == "1" ]; then
    # Локальная директория
    read -p "Enter directory path [/home/romaxa/share]: " SHARE_DIR
    SHARE_DIR=${SHARE_DIR:-/home/romaxa/share}

elif [ "$SOURCE_TYPE" == "2" ]; then
    # Samba share
    echo "Samba/CIFS Share Setup"
    read -p "Enter server IP: " SERVER_IP
    read -p "Enter share name [share]: " SHARE_NAME
    SHARE_NAME=${SHARE_NAME:-share}
    read -p "Enter username (or press Enter for guest): " USERNAME

    MOUNT_POINT="/tmp/screenshot_share_$$"
    mkdir -p "$MOUNT_POINT"

    echo "Mounting Samba share..."
    if [ -z "$USERNAME" ]; then
        # Guest access
        sudo mount -t cifs "//$SERVER_IP/$SHARE_NAME" "$MOUNT_POINT" -o guest,uid=$(id -u),gid=$(id -g)
    else
        # With credentials
        sudo mount -t cifs "//$SERVER_IP/$SHARE_NAME" "$MOUNT_POINT" -o username=$USERNAME,uid=$(id -u),gid=$(id -g)
    fi

    if [ $? -eq 0 ]; then
        echo "✓ Mounted successfully"
        SHARE_DIR="$MOUNT_POINT"
    else
        echo "✗ Failed to mount share"
        exit 1
    fi

elif [ "$SOURCE_TYPE" == "3" ]; then
    # SSH/SFTP
    echo "SSH/SFTP Mount Setup"
    read -p "Enter SSH user@host: " SSH_HOST
    read -p "Enter remote path [/home/romaxa/share]: " REMOTE_PATH
    REMOTE_PATH=${REMOTE_PATH:-/home/romaxa/share}

    MOUNT_POINT="/tmp/screenshot_share_$$"
    mkdir -p "$MOUNT_POINT"

    # Проверяем наличие sshfs
    if ! command -v sshfs &> /dev/null; then
        echo "Installing sshfs..."
        sudo apt-get install -y sshfs
    fi

    echo "Mounting SSH filesystem..."
    sshfs "$SSH_HOST:$REMOTE_PATH" "$MOUNT_POINT"

    if [ $? -eq 0 ]; then
        echo "✓ Mounted successfully"
        SHARE_DIR="$MOUNT_POINT"
    else
        echo "✗ Failed to mount SSH filesystem"
        exit 1
    fi
else
    echo "Invalid option"
    exit 1
fi

LATEST_LINK="${SHARE_DIR}/latest_screenshot.png"

echo ""
echo "Monitoring: $SHARE_DIR"
echo "Latest screenshot link: $LATEST_LINK"
echo ""
echo "Press Ctrl+C to stop"
echo ""

if [ ! -d "$SHARE_DIR" ]; then
    echo "Error: Directory $SHARE_DIR does not exist!"
    exit 1
fi

# Проверяем наличие feh (быстрый просмотрщик изображений)
if ! command -v feh &> /dev/null; then
    echo "Installing feh (image viewer)..."
    sudo apt-get install -y feh
fi

# Функция очистки при выходе
cleanup() {
    echo ""
    echo "Cleaning up..."
    if [ "$SOURCE_TYPE" == "2" ] || [ "$SOURCE_TYPE" == "3" ]; then
        if [ -n "$MOUNT_POINT" ]; then
            echo "Unmounting $MOUNT_POINT..."
            if [ "$SOURCE_TYPE" == "2" ]; then
                sudo umount "$MOUNT_POINT"
            else
                fusermount -u "$MOUNT_POINT"
            fi
            rmdir "$MOUNT_POINT" 2>/dev/null
        fi
    fi
    echo "Done"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Ждём пока появится первый скриншот
echo "Waiting for first screenshot..."
while [ ! -f "$LATEST_LINK" ]; do
    sleep 1
    printf "."
done
echo ""
echo ""

# Показываем статистику
FRAME_COUNT=0
LAST_FILE=""
START_TIME=$(date +%s)
export LC_NUMERIC=C

echo "Viewer starting..."
echo ""

# Создаём временную копию для отображения
TEMP_DISPLAY="/tmp/display_screenshot_$$.png"

# Мониторим изменения и обновляем просмотрщик
FEH_PID=""

while true; do
    # Получаем реальный путь к файлу (разрешаем симлинк)
    if [ -L "$LATEST_LINK" ]; then
        REAL_FILE=$(readlink -f "$LATEST_LINK")
    elif [ -f "$LATEST_LINK" ]; then
        REAL_FILE="$LATEST_LINK"
    else
        sleep 0.5
        continue
    fi

    # Проверяем изменился ли файл
    if [ "$REAL_FILE" != "$LAST_FILE" ] && [ -f "$REAL_FILE" ]; then
        # Копируем новое изображение
        cp "$REAL_FILE" "$TEMP_DISPLAY" 2>/dev/null

        if [ -f "$TEMP_DISPLAY" ]; then
            FRAME_COUNT=$((FRAME_COUNT + 1))
            ELAPSED=$(($(date +%s) - START_TIME))

            if [ $ELAPSED -gt 0 ]; then
                AVG_FPS=$(echo "scale=2; $FRAME_COUNT / $ELAPSED" | bc)
            else
                AVG_FPS="0.00"
            fi

            SIZE_MB=$(echo "scale=2; $(stat -c%s "$REAL_FILE") / 1024 / 1024" | bc)

            printf "\r[%s] Updates: %d | Time: %ds | Avg FPS: %s | Size: %s MB  " \
                "$(date +%H:%M:%S)" $FRAME_COUNT $ELAPSED $AVG_FPS $SIZE_MB

            # Если feh ещё не запущен или закрылся, запускаем заново
            if [ -z "$FEH_PID" ] || ! kill -0 $FEH_PID 2>/dev/null; then
                # Убиваем старый процесс если есть
                [ -n "$FEH_PID" ] && kill $FEH_PID 2>/dev/null

                # Запускаем feh с автообновлением
                feh -F -Z --reload 0.3 "$TEMP_DISPLAY" &
                FEH_PID=$!
            fi

            LAST_FILE="$REAL_FILE"
        fi
    fi

    # Проверяем что feh ещё работает
    if [ -n "$FEH_PID" ] && ! kill -0 $FEH_PID 2>/dev/null; then
        echo ""
        echo "Viewer closed"
        rm -f "$TEMP_DISPLAY"
        cleanup
    fi

    sleep 0.3
done

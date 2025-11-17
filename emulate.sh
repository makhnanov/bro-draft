#!/bin/bash

# Проверка существования файла password.txt
if [ ! -f "password.txt" ]; then
    echo "Ошибка: файл password.txt не найден"
    exit 1
fi

# Проверка установки xdotool
if ! command -v xdotool &> /dev/null; then
    echo "Ошибка: xdotool не установлен"
    echo "Установите его командой: sudo apt-get install xdotool"
    exit 1
fi

# Задержка перед началом эмуляции (5 секунд для переключения на нужное окно)
echo "Переключитесь на нужное окно. Начало эмуляции через 5 секунд..."
sleep 5

# Чтение содержимого файла и эмуляция ввода
password=$(cat password.txt)
xdotool type --delay 100 "$password"

echo "Эмуляция завершена"

#!/bin/bash

# Скрипт для получения статистики использования Claude Code
# Запускает /usage команду, записывает вывод в файл и завершает через 30 секунд

OUTPUT_FILE="claude_usage_$(date +%Y%m%d_%H%M%S).txt"

echo "Запуск мониторинга использования Claude Code..."
echo "Вывод будет сохранен в: $OUTPUT_FILE"

# Создаем временный файл для ввода команды
TEMP_INPUT=$(mktemp)
echo "/usage" > "$TEMP_INPUT"
echo "exit" >> "$TEMP_INPUT"

# Запускаем claude в фоне с перенаправлением вывода
timeout 30s claude < "$TEMP_INPUT" > "$OUTPUT_FILE" 2>&1 &
CLAUDE_PID=$!

# Ждем завершения (максимум 30 секунд)
wait $CLAUDE_PID 2>/dev/null

# Удаляем временный файл
rm -f "$TEMP_INPUT"

# Проверяем, что файл создан
if [ -f "$OUTPUT_FILE" ]; then
    echo "✓ Вывод успешно сохранен в: $OUTPUT_FILE"
    echo ""
    echo "Содержимое:"
    cat "$OUTPUT_FILE"
else
    echo "✗ Ошибка: файл не был создан"
    exit 1
fi

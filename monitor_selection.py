#!/usr/bin/env python3
"""
Скрипт для отслеживания выделенного текста в реальном времени
Работает на Linux с X11
"""

import subprocess
import time
import sys

def get_selected_text():
    """Получить текущий выделенный текст из PRIMARY selection"""
    try:
        result = subprocess.run(
            ['xclip', '-o', '-selection', 'primary'],
            capture_output=True,
            text=True,
            timeout=0.5
        )
        return result.stdout if result.returncode == 0 else None
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return None

def clear_line():
    """Очистить текущую строку в терминале"""
    sys.stdout.write('\r\033[K')
    sys.stdout.flush()

def main():
    print("Мониторинг выделенного текста (Ctrl+C для выхода)")
    print("-" * 60)

    last_text = None

    try:
        while True:
            current_text = get_selected_text()

            if current_text != last_text:
                clear_line()

                if current_text and current_text.strip():
                    # Ограничиваем длину отображаемого текста
                    display_text = current_text.replace('\n', '\\n')
                    if len(display_text) > 100:
                        display_text = display_text[:100] + '...'

                    print(f"\r✓ Выделено: [{len(current_text)} симв.] {display_text}", end='', flush=True)
                else:
                    print(f"\r✗ Текст не выделен", end='', flush=True)

                last_text = current_text

            time.sleep(0.1)  # Проверка каждые 100ms

    except KeyboardInterrupt:
        print("\n\nМониторинг остановлен")

if __name__ == '__main__':
    main()

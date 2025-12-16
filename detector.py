import sounddevice as sd
import numpy as np
import sys

# --- Настройки ---
# Порог тишины. 
# 0.0001 — это очень тихий шум. 
# Hardware Mute обычно выдает чистый 0.0.
THRESHOLD = 0.00005 

# Сколько секунд тишины должно пройти, чтобы считать микрофон выключенным
# (чтобы избежать ложных срабатываний при коротких паузах в речи)
SILENCE_LIMIT = 0.3 

print("--- Мониторинг звукового потока запущен ---")
print("Используется входное устройство по умолчанию.")
print("Нажмите Ctrl+C для выхода.")

class MicState:
    is_muted = False
    silence_counter = 0

state = MicState()

def callback(indata, frames, time, status):
    if status:
        print(status, file=sys.stderr)
    
    # Вычисляем среднеквадратичную амплитуду (RMS)
    rms = np.sqrt(np.mean(indata**2))
    
    if rms < THRESHOLD:
        state.silence_counter += frames / 44100 # Увеличиваем счетчик времени тишины
        if state.silence_counter >= SILENCE_LIMIT and not state.is_muted:
            print("\n[ СОБЫТИЕ ] Микрофон выключен кнопкой (Hardware Mute)")
            state.is_muted = True
    else:
        state.silence_counter = 0
        if state.is_muted:
            print("\n[ СОБЫТИЕ ] Микрофон снова включен (Live)")
            state.is_muted = False
    
    # Визуальный индикатор в реальном времени
    # Раскомментируйте строку ниже, чтобы видеть уровень шума для калибровки THRESHOLD
    # print(f"Текущий уровень RMS: {rms:.8f}    ", end='\r')

try:
    # Запускаем поток захвата
    with sd.InputStream(callback=callback, channels=1, samplerate=44100):
        while True:
            sd.sleep(100)
except KeyboardInterrupt:
    print("\nМониторинг остановлен.")
except Exception as e:
    print(f"\nПроизошла ошибка: {e}")


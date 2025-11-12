# BRO - Системный монитор

Приложение для мониторинга температуры CPU, скорости интернета и курса Bitcoin.

## Быстрый старт

### 1. Установка зависимостей

```bash
# Создать виртуальное окружение Python и установить зависимости
make setup-venv

# Или установить speedtest (один из вариантов):
# Вариант 1: Speedtest CLI от Ookla (рекомендуется)
curl -s https://packagecloud.io/install/repositories/ookla/speedtest-cli/script.deb.sh | sudo bash
sudo apt-get install speedtest

# Вариант 2: speedtest-cli (Python)
pip3 install speedtest-cli
```

### 2. Запуск приложения

```bash
# Автоматический запуск (API + Приложение)
./start.sh

# Или вручную:
# Терминал 1 - API сервер
make start-api

# Терминал 2 - Приложение
make start-development
```

## Архитектура Speedtest API

**Текущая реализация:**
- Frontend (Vue) → HTTP REST API (Python Flask) → speedtest-cli → JSON ответ
- Аналогично тому, как работает Bitcoin через API

**Преимущества:**
- Независимость от Tauri
- Простота тестирования
- Масштабируемость
- Единый подход для внешних данных (Bitcoin и Speedtest через API)

## API Endpoints

### GET /api/speedtest
Выполняет тест скорости интернета

**Ответ:**
```json
{
  "ping": 10.5,
  "download": 125.3,
  "upload": 45.7
}
```

### GET /api/health
Проверка работоспособности API

**Ответ:**
```json
{
  "status": "ok"
}
```

## Команды Makefile

- `make setup-venv` - создать виртуальное окружение и установить зависимости
- `make start-api` - запустить только API сервер
- `make start-development` - запустить только Tauri приложение
- `make start-all` - запустить API и приложение вместе

## Структура проекта

```
.
├── speedtest-api.py       # REST API сервер на Flask
├── requirements.txt       # Python зависимости
├── start.sh              # Скрипт автоматического запуска
├── Makefile              # Команды для сборки и запуска
├── SPEEDTEST_API.md      # Подробная документация API
├── src/
│   └── App.vue           # Frontend (строки 107-129 - Speedtest)
└── src-tauri/
    └── src/
        └── lib.rs        # Rust backend

```

## Особенности

- **Температура CPU**: обновляется каждую секунду (Rust → sysinfo)
- **Speedtest**: обновляется каждые 30 секунд (Frontend → Python API → speedtest-cli)
- **Bitcoin**: обновляется каждые 20 секунд в DEV режиме / 10 минут в продакшене (Frontend → CoinGecko API)
- **Время**: показывается на час меньше (UTC-1)

## Горячие клавиши

- `F5` - перезагрузить приложение
- `F11` - полноэкранный режим
- `F12` - открыть/закрыть DevTools

## Устранение проблем

### Ошибка "TypeError: Load failed"

Эта ошибка возникает когда API сервер не запущен или недоступен.

**Решение:**

1. Убедитесь, что API сервер запущен:
   ```bash
   curl http://127.0.0.1:5000/api/health
   ```

2. Если API не отвечает, запустите его:
   ```bash
   make start-api
   # или
   ./venv/bin/python speedtest-api.py
   ```

3. Перезапустите приложение

### Другие проблемы

Смотрите подробную документацию в [SPEEDTEST_API.md](SPEEDTEST_API.md)

## Требования

- Python 3.8+
- Node.js & pnpm
- Rust (для сборки Tauri)
- speedtest или speedtest-cli

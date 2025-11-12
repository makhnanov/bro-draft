# Speedtest Integration

## Описание

Приложение использует **Cloudflare Speedtest API** для измерения скорости интернет-соединения.

## Преимущества Cloudflare Speedtest

✅ **Не требует локального сервера** - никакого Python/Flask API
✅ **Официальное решение** - поддерживается Cloudflare
✅ **Бесплатно** - полностью бесплатное использование
✅ **Надежно** - использует edge network Cloudflare
✅ **Просто** - встроено в приложение через npm пакет

## Архитектура

**Новая реализация:**
```
Frontend (Vue) → @cloudflare/speedtest (npm) → Cloudflare Edge Network → Результаты
```

**Старая реализация (удалена):**
```
Frontend (Vue) → Python Flask API → speedtest-cli → Результаты
```

## Установка

### Зависимости

Установите npm пакет (уже включен в package.json):

```bash
pnpm install @cloudflare/speedtest
```

### Запуск приложения

```bash
# Простой запуск
./start.sh

# Или через Makefile
make start-development
```

## Использование

Speedtest запускается автоматически:
- При запуске приложения (через 2 секунды после загрузки UI)
- Каждые 30 секунд для обновления данных

Результаты отображаются в двух спидометрах:
- **Download** - скорость загрузки (↓)
- **Upload** - скорость отдачи (↑)

## Технические детали

### Код интеграции (src/App.vue)

```typescript
import SpeedTest from "@cloudflare/speedtest";

async function checkNetworkSpeed() {
  const speedtest = new SpeedTest();

  // Обработчик обновления во время теста
  speedtest.onResultsChange = ({ type }) => {
    const summary = speedtest.results.getSummary();
    if (type === 'download') downloadSpeed.value = summary.download;
    if (type === 'upload') uploadSpeed.value = summary.upload;
  };

  // Запуск теста
  await speedtest.play();
}
```

### Конфигурация безопасности

Tauri настроен для работы с Cloudflare API:

**tauri.conf.json:**
```json
{
  "security": {
    "csp": "connect-src 'self' https://api.coingecko.com https://*.cloudflare.com https://speed.cloudflare.com"
  }
}
```

**capabilities/default.json:**
```json
{
  "remote": {
    "urls": ["https://*.cloudflare.com", "https://speed.cloudflare.com"]
  }
}
```

## Отличия от предыдущей версии

| Параметр | Старая версия | Новая версия |
|----------|---------------|--------------|
| Локальный сервер | ✅ Требуется Python Flask | ❌ Не требуется |
| Зависимости | Python, Flask, speedtest-cli | Только npm пакет |
| Запуск | start.sh запускает 2 процесса | start.sh запускает только Tauri |
| Надежность | Зависит от локального speedtest-cli | Использует Cloudflare Edge |
| Скорость запуска | ~3 сек (запуск API) | Мгновенно |

## Удаленные файлы

Следующие файлы больше не нужны и были удалены:
- `speedtest-api.py` - Python Flask API сервер
- `requirements.txt` - зависимости Python
- `SPEEDTEST_API.md` - старая документация

## Ресурсы

- **NPM пакет:** [@cloudflare/speedtest](https://www.npmjs.com/package/@cloudflare/speedtest)
- **GitHub:** [cloudflare/speedtest](https://github.com/cloudflare/speedtest)
- **Документация:** [Cloudflare Speed Test](https://developers.cloudflare.com/speed/speed-test/)

## Устранение проблем

### Speedtest не работает

1. Проверьте консоль браузера (F12) на наличие ошибок
2. Убедитесь, что есть интернет-соединение
3. Проверьте, что пакет установлен: `pnpm list @cloudflare/speedtest`

### Медленные результаты

Cloudflare Speedtest выполняет полноценный тест, который может занять 10-30 секунд. Это нормально.

### Ошибка CSP/CORS

Если видите ошибки безопасности, убедитесь, что конфигурация Tauri обновлена и включает Cloudflare домены.

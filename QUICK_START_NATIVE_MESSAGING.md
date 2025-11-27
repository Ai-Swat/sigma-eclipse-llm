# 🚀 Quick Start: Native Messaging для Chrome Extension

Быстрая инструкция по настройке связи между вашим расширением Chrome и приложением Sigma Eclipse.

## Шаг 1: Сборка Native Host (5 минут)

```bash
cd src-tauri
cargo build --release --bin sigma-eclipse-host
```

**Результат:** Бинарник создан в `src-tauri/target/release/sigma-eclipse-host`

**Для production:** При сборке основного приложения, также соберите host:
```bash
cargo build --release --bin sigma-eclipse-host
# Скопируйте бинарник в ту же папку что и основной exe
```

## Шаг 2: Установка манифеста (1 минута)

```bash
./scripts/install-native-messaging-host.sh
```

Следуйте инструкциям на экране. Скрипт:
- Найдёт бинарник
- Создаст манифест
- Установит его для Chrome/Edge

**Альтернатива (вручную):**

Создайте файл:
- **macOS**: `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.sigma_eclipse.host.json`

С содержимым:
```json
{
  "name": "com.sigma_eclipse.host",
  "description": "Sigma Eclipse LLM Native Messaging Host",
  "path": "/путь/к/sigma-eclipse-host",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://ВАШ_EXTENSION_ID/"
  ]
}
```

## Шаг 3: Тестирование (5 минут)

### Создайте своё тестовое расширение

Создайте папку для расширения с файлами:

**manifest.json:**
```json
{
  "manifest_version": 3,
  "name": "Sigma Eclipse Test",
  "version": "1.0.0",
  "permissions": ["nativeMessaging"],
  "background": {
    "service_worker": "background.js"
  }
}
```

**background.js:**
```javascript
// Подключение к хосту
const port = chrome.runtime.connectNative('com.sigma_eclipse.host');

// Отправка команды
port.postMessage({
  id: '1',
  command: 'get_server_status',
  params: {}
});

// Получение ответа
port.onMessage.addListener((message) => {
  console.log('Ответ:', message);
  // { id: '1', success: true, data: { is_running: false, ... } }
});
```

## Доступные команды

### 1. Получить статус сервера
```javascript
port.postMessage({
  id: '1',
  command: 'get_server_status',
  params: {}
});
// Ответ: { is_running: true/false, pid: 12345, message: "..." }
```

### 2. Запустить сервер
```javascript
port.postMessage({
  id: '2',
  command: 'start_server',
  params: {
    port: 8080,
    ctx_size: 8192,
    gpu_layers: 0
  }
});
// Ответ: { message: "Server started...", pid: 12345, port: 8080 }
```

### 3. Остановить сервер
```javascript
port.postMessage({
  id: '3',
  command: 'stop_server',
  params: {}
});
// Ответ: { message: "Server stopped" }
```

### 4. Проверить статус загрузки
```javascript
port.postMessage({
  id: '4',
  command: 'isDownloading',
  params: {}
});
// Ответ: { is_downloading: false, progress: null }
```

## Отладка

### Логи Native Host
```bash
# Запустите хост вручную для просмотра логов
echo '{"id":"1","command":"get_server_status","params":{}}' | \
  /path/to/sigma-eclipse-host
```

### Логи расширения
1. `chrome://extensions/`
2. Найдите ваше расширение
3. Кликните "service worker" или "фоновая страница"
4. Смотрите консоль

### Частые ошибки

| Ошибка | Решение |
|--------|---------|
| "Specified native messaging host not found" | Проверьте путь в манифесте и что файл существует |
| "Access to the specified native messaging host is forbidden" | Обновите `allowed_origins` с правильным Extension ID |
| "Failed to start native messaging host" | Убедитесь что бинарник исполняемый: `chmod +x sigma-eclipse-host` |

## Полная документация

Смотрите [NATIVE_MESSAGING.md](NATIVE_MESSAGING.md) для детальной информации:
- Архитектура и протокол
- Все доступные команды
- Примеры кода
- Интеграция в production

## Структура файлов

```
sigma-eclipse/
├── src-tauri/
│   ├── src/
│   │   ├── bin/
│   │   │   └── native_messaging_host.rs  # Native host binary
│   │   ├── ipc_state.rs                  # IPC state management
│   │   ├── server_manager.rs             # Shared server logic
│   │   └── ...
│   └── target/release/
│       └── sigma-eclipse-host             # Скомпилированный бинарник
├── scripts/
│   └── install-native-messaging-host.sh  # Скрипт установки
├── native-messaging/
│   └── com.sigma-eclipse.host.json        # Шаблон манифеста
├── NATIVE_MESSAGING.md                   # Полная документация
├── QUICK_START_NATIVE_MESSAGING.md       # Этот файл
└── CHANGELOG_NATIVE_MESSAGING.md         # Changelog
```

## Следующие шаги

1. ✅ Создайте своё расширение
2. ✅ Проверьте все команды
3. ✅ Интегрируйте в production
4. ✅ Прочитайте полную документацию

---

**Вопросы?** Смотрите [NATIVE_MESSAGING.md](NATIVE_MESSAGING.md) для детальной информации и примеров


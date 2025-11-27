# Native Messaging Integration - Changelog

## Что было добавлено

### 1. Native Messaging Host Binary

**Файл:** `src-tauri/src/bin/native_messaging_host.rs`

Отдельный исполняемый бинарник, который:
- Реализует Chrome Native Messaging Protocol
- Читает команды из stdin, отправляет ответы в stdout
- Использует общий модуль `server_manager` для управления сервером
- Поддерживает команды: `start_server`, `stop_server`, `get_server_status`, `isDownloading`

### 2. Server Manager Module

**Файл:** `src-tauri/src/server_manager.rs`

**[НОВЫЙ]** Общий модуль управления сервером:
- Устраняет дублирование кода между `server.rs` и `native_messaging_host.rs`
- Функции: `start_server_process()`, `stop_server_by_pid()`, `get_status()`, `check_server_running()`
- Используется как в Tauri, так и в Native Host
- Единая точка входа для управления LLM сервером

### 3. IPC State Management Module

**Файл:** `src-tauri/src/ipc_state.rs`

Модуль для межпроцессного взаимодействия:
- Хранит состояние в JSON файле (`~/Library/Application Support/sigma-shield/ipc_state.json`)
- Позволяет Native Host и Tauri приложению обмениваться данными
- Отслеживает статус сервера, PID процесса, параметры (port, ctx_size, gpu_layers), прогресс загрузки

### 3. Манифесты и скрипты установки

**Файлы:**
- `native-messaging/com.sigma-shield.host.json` - Шаблон манифеста
- `scripts/install-native-messaging-host.sh` - Скрипт автоматической установки

Автоматизируют установку Native Messaging Host:
- Определяют платформу (macOS/Linux)
- Находят бинарник
- Создают и устанавливают манифест
- Поддерживают Chrome и Edge

### 4. Примеры кода

**[ОБНОВЛЕНО]** Полные примеры перенесены в документацию:
- Background Service Worker для Manifest V3
- Helper класс `SigmaShieldClient`
- Promise-based обёртка для команд
- Обработка ошибок и переподключения
- Все примеры из бывшего тестового расширения теперь в `NATIVE_MESSAGING.md`

### 5. Документация

**Файлы:**
- `NATIVE_MESSAGING.md` - Полная техническая документация
- `QUICK_START_NATIVE_MESSAGING.md` - Быстрый старт
- `CHANGELOG_NATIVE_MESSAGING.md` - Этот файл

Детальное описание:
- Архитектуры и протокола
- Всех доступных команд
- Примеров использования
- Отладки и troubleshooting

### 6. Обновления существующих файлов

**[РЕФАКТОРИНГ]** Изменённые файлы:
- `src-tauri/Cargo.toml` - Добавлен новый бинарник `[[bin]]`
- `src-tauri/src/lib.rs` - Экспортированы модули `ipc_state` и `server_manager`
- `src-tauri/src/server.rs` - Отрефакторен для использования `server_manager`, устранено дублирование
- Оба процесса (Tauri и Native Host) теперь используют общую логику через IPC state

## Структура проекта после изменений

```
sigma-shield/
├── src-tauri/
│   ├── src/
│   │   ├── bin/
│   │   │   └── native_messaging_host.rs    # ← НОВЫЙ: Native host (отрефакторен)
│   │   ├── ipc_state.rs                    # ← НОВЫЙ: IPC модуль
│   │   ├── server_manager.rs               # ← НОВЫЙ: Общая логика сервера
│   │   ├── server.rs                       # ОТРЕФАКТОРЕН: использует server_manager
│   │   ├── lib.rs                          # ИЗМЕНЁН: экспорт модулей
│   │   └── ...
│   ├── Cargo.toml                          # ИЗМЕНЁН: добавлен [[bin]]
│   └── target/release/
│       └── sigma-shield-host               # ← НОВЫЙ: бинарник
├── scripts/
│   └── install-native-messaging-host.sh    # ← НОВЫЙ: установка
├── native-messaging/
│   └── com.sigma-shield.host.json          # ← НОВЫЙ: шаблон манифеста
├── NATIVE_MESSAGING.md                     # ← НОВЫЙ: документация + примеры
├── QUICK_START_NATIVE_MESSAGING.md         # ← НОВЫЙ: quick start
└── CHANGELOG_NATIVE_MESSAGING.md           # ← НОВЫЙ: этот файл
```

## Как это работает

```
┌─────────────────────────────────────────────────────────────────┐
│                      Browser (Chrome/Edge)                      │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Your Chrome Extension                       │  │
│  │                                                          │  │
│  │  chrome.runtime.connectNative('com.sigma_shield.host')  │  │
│  └────────────────────────┬─────────────────────────────────┘  │
└───────────────────────────┼─────────────────────────────────────┘
                            │ Native Messaging Protocol
                            │ (stdin/stdout, JSON messages)
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│              sigma-shield-host (Rust binary)                    │
│                                                                 │
│  • Reads commands from stdin                                   │
│  • Processes commands (start/stop/status)                      │
│  • Manages LLM server process                                  │
│  • Writes responses to stdout                                  │
│  • Updates IPC state file                                      │
└────────────────────────┬────────────────────────────────────────┘
                         │ File-based IPC
                         │ (ipc_state.json)
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│         Sigma Shield Tauri App (optional)                       │
│                                                                 │
│  • Can read/write same IPC state                               │
│  • Shares information with Native Host                         │
│  • Both can manage server independently                        │
└─────────────────────────────────────────────────────────────────┘
```

## Поддерживаемые команды

| Команда | Описание | Параметры |
|---------|----------|-----------|
| `start_server` | Запустить LLM сервер | port, ctx_size, gpu_layers |
| `stop_server` | Остановить LLM сервер | - |
| `get_server_status` | Получить статус сервера | - |
| `isDownloading` | Проверить статус загрузки | - |

## Зависимости

**Новые зависимости:** Нет (используются только существующие)

**Используемые crates:**
- `serde`, `serde_json` - JSON сериализация
- `anyhow` - Обработка ошибок
- `dirs` - Системные директории
- `libc` (Unix) - Управление процессами

## Совместимость

- ✅ **macOS**: Полная поддержка
- ✅ **Linux**: Полная поддержка  
- ⚠️ **Windows**: Требует ручной настройки реестра (документировано)

- ✅ **Chrome**: Полная поддержка
- ✅ **Edge (Chromium)**: Полная поддержка
- ❌ **Firefox**: Не поддерживается (другой протокол)
- ❌ **Safari**: Не поддерживается

## Тестирование

Для тестирования:

1. Соберите бинарник:
   ```bash
   cargo build --release --bin sigma-shield-host
   ```

2. Установите манифест:
   ```bash
   ./scripts/install-native-messaging-host.sh
   ```

3. Загрузите `example-extension` в Chrome

4. Протестируйте команды через UI

## Безопасность

- ✅ Только расширения из `allowed_origins` могут подключиться
- ✅ Коммуникация только через локальный stdio
- ✅ Нет сетевого доступа у хоста
- ✅ Каждое соединение = отдельный процесс

## Production Deployment

Для deployment в production:

1. Включите бинарник в bundle приложения
2. Установите манифест при установке приложения
3. Обновите `path` в манифесте на путь к установленному бинарнику
4. Добавьте реальный Extension ID в `allowed_origins`

См. раздел "Building for Distribution" в `NATIVE_MESSAGING.md`

## Будущие улучшения (опционально)

- [ ] Добавить команды для управления загрузками
- [ ] Поддержка Firefox Native Messaging
- [ ] WebSocket альтернатива для двусторонней связи
- [ ] Автоматическая установка манифеста при первом запуске
- [ ] Signing бинарника для macOS Gatekeeper

## Версия

**Дата:** 2025-11-27
**Версия приложения:** 0.1.0
**Extension ID (test):** lidcgfpdpjpeambpilgmllbefcikkglh

## Авторы

Создано для проекта Sigma Shield LLM


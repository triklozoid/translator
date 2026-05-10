# План реализации Translator Android

> Статус: проектирование | 2026-05-09

## 0. Обзор: что портируем из десктопной версии

Десктопный Translator (Rust/GTK) состоит из следующих логических блоков:

| Блок | Rust-реализация | Android-аналог |
|---|---|---|
| **Конфигурация** | `config.rs` — TOML-файл, `load_config()` / `save_config()` | DataStore Preferences (typed key-value) |
| **API-ключ** | `resolve_api_key()` — авто-выбор env var по URL | DataStore + ручной ввод в настройках |
| **Детекция языка** | lingua-rs (Rust crate) | ML Kit Language Identification |
| **Алгоритм выбора языка** | `choose_target_language()` | Портируется 1:1 на Kotlin |
| **Перевод** | `translation.rs` — llm_connector → OpenRouter | Retrofit + OkHttp → тот же API |
| **Буфер обмена** | GTK `Clipboard` → `read_text_future()` | `ClipboardManager.addPrimaryClipChangedListener()` |
| **UI** | GTK окно (900×800) | Compose: экран настроек + плавающая панель |
| **Логирование** | `logger.rs` → JSON-файл | Опционально, во v2 |

---

## Фазы реализации

### Фаза 1: Фундамент — скелет проекта и конфигурация  🔴 MUST

**Что**: минимальный собираемый APK с экраном настроек. Без перевода, без бабла.

```
android/app/src/main/java/com/translator/android/
├── TranslatorApp.kt              # Application class
├── MainActivity.kt               # Экран настроек (уже есть заготовка)
├── data/
│   ├── SettingsDataStore.kt      # DataStore для конфига
│   └── model/
│       └── Language.kt           # Enum языков (ISO-коды)
└── ui/
    └── theme/
        └── Theme.kt              # Material3 тема
```

#### Задача 1.1 — Модель языка
- [ ] `Language.kt` — enum с ISO-кодами (EN, RU, FR, IT, PL...)
- [ ] Функции: `byIsoCode()`, `isoCode()`, `displayName()`
- [ ] Ограниченный список для v1 (10–15 языков), расширяемый

#### Задача 1.2 — DataStore для настроек
- [ ] `SettingsDataStore.kt` — обёртка над `Preferences DataStore`
- [ ] Поля: `apiKey`, `apiUrl`, `modelVersion`, `primaryLang`, `secondaryLang`, `allTargetLangs`, `lastTargetLang`, `showBubble`
- [ ] Flow-based чтение, suspend-запись
- [ ] Значения по умолчанию (как `Config::default()` в Rust)

#### Задача 1.3 — Экран настроек
- [ ] Переработать `MainActivity.kt` в полноценный экран настроек
- [ ] Секции: API (ключ, URL, модель), Языки (primary/secondary/список), Бабл (тумблер)
- [ ] Сохранение через SettingsDataStore
- [ ] Валидация: API ключ не пустой перед включением бабла

#### Задача 1.4 — Application class
- [ ] `TranslatorApp.kt` — инициализация DataStore
- [ ] Hilt/manual DI (решить: Hilt или ручной DI)

**Результат фазы 1**: APK открывается → экран настроек → данные сохраняются между запусками.

---

### Фаза 2: Перевод — API-слой и логика  🔴 MUST

**Что**: кнопка «Перевести» на экране настроек работает как proof-of-concept.

#### Задача 2.1 — OpenRouter API клиент
- [ ] `api/OpenRouterApi.kt` — Retrofit interface
- [ ] Data-классы: `ChatRequest`, `Message`, `ChatResponse`, `Choice`
- [ ] `api/ApiClient.kt` — singleton Retrofit instance
- [ ] Interceptor: добавление `Authorization: Bearer`, `HTTP-Referer`

#### Задача 2.2 — Языковая детекция
- [ ] `language/LanguageDetector.kt` — обёртка над ML Kit Language Identification
- [ ] `detectLanguage(text: String): Language?`
- [ ] Таймаут 2 секунды (как в Rust-версии)
- [ ] Использовать первые 100 символов для детекции
- [ ] Fallback: если ML Kit недоступен — возврат null → перевод на primary

#### Задача 2.3 — Алгоритм выбора языка
- [ ] `language/LanguageSelector.kt` — порт `choose_target_language()`
- [ ] Вход: `sourceLang: Language?`, `primaryLang`, `secondaryLang`, `lastLang`
- [ ] Выход: `targetLang: Language`
- [ ] Unit-тесты для всех трёх веток алгоритма

#### Задача 2.4 — Use Case: перевод
- [ ] `domain/TranslateUseCase.kt`
- [ ] Вход: текст, targetLanguage, apiKey, apiUrl, model
- [ ] Выход: `Result<String, TranslationError>`
- [ ] Обработка ошибок: empty text, network, auth, rate limit
- [ ] Промпт: "Translate the following text into {lang}. Provide only the translation, nothing else."

#### Задача 2.5 — ViewModel для экрана настроек
- [ ] `ui/settings/SettingsViewModel.kt`
- [ ] Поле для ручного ввода текста
- [ ] Кнопка «Перевести» → вызывает TranslateUseCase
- [ ] Показ результата / ошибки

**Результат фазы 2**: на экране настроек можно ввести текст → нажать «Перевести» → увидеть результат от OpenRouter.

---

### Фаза 3: Буфер обмена — авто-захват текста  🔴 MUST

**Что**: foreground service слушает буфер обмена. Скопировал текст → он сохранён и готов к переводу.

#### Задача 3.1 — Clipboard Monitor Service
- [ ] `service/ClipboardService.kt` — Foreground Service
- [ ] `onCreate()`: регистрация `ClipboardManager.OnPrimaryClipChangedListener`
- [ ] При изменении буфера: извлечь текст → сохранить в DataStore
- [ ] Проверки: текст не пустой, не длиннее 5000 символов
- [ ] Нотификация foreground service («Translator активен»)
- [ ] Жизненный цикл: старт/стоп по тумблеру из настроек

#### Задача 3.2 — Передача текста в UI
- [ ] Flow в DataStore: `clipboardTextFlow: Flow<String?>`
- [ ] UI подписывается на Flow — текст обновляется реактивно

#### Задача 3.3 — Android 14+ совместимость
- [ ] `foregroundServiceType="specialUse"` в манифесте
- [ ] `PROPERTY_SPECIAL_USE_FGS_SUBTYPE`
- [ ] `POST_NOTIFICATIONS` permission для Android 13+

**Результат фазы 3**: скопировал текст в любом приложении → UI видит этот текст.

---

### Фаза 4: Плавающий бабл — Floating Bubble  🔴 MUST

**Что**: иконка поверх всех приложений. Нажал → раскрылась панель перевода.

#### Задача 4.1 — WindowManager overlay
- [ ] `service/OverlayService.kt` — Foreground Service (или объединить с ClipboardService)
- [ ] `WindowManager.addView()` — добавить ComposeView поверх всех окон
- [ ] `TYPE_APPLICATION_OVERLAY` + `FLAG_NOT_FOCUSABLE`
- [ ] Запрос `SYSTEM_ALERT_WINDOW` permission

#### Задача 4.2 — Bubble View (компонент)
- [ ] `ui/bubble/BubbleView.kt` — круглая иконка (56dp)
- [ ] Перетаскивание (drag & drop) по краям экрана
- [ ] Анимация: пульсация при новом тексте в буфере
- [ ] Прилипание к краю после окончания перетаскивания

#### Задача 4.3 — Translation Panel (развёрнутый бабл)
- [ ] `ui/panel/TranslationPanel.kt`
- [ ] Компактная панель (~60% экрана)
- [ ] Элементы: source text, language selector (EN → RU), результат, кнопки
- [ ] Авто-детекция языка при открытии
- [ ] Авто-перевод при открытии (если текст уже есть из буфера)

#### Задача 4.4 — Связка Bubble ↔ Panel
- [ ] Состояние: `isExpanded: Boolean`
- [ ] Тап по свёрнутому баблу → развернуть панель
- [ ] Кнопка «Свернуть» → скрыть панель, показать бабл
- [ ] Кнопка «Копировать» → скопировать результат, свернуть

#### Задача 4.5 — Интеграция с ClipboardService
- [ ] При новом тексте в буфере: бабл пульсирует
- [ ] Если панель развёрнута: авто-перевод нового текста
- [ ] Если свёрнута: просто индикация (пульсация)

**Результат фазы 4**: бабл висит поверх приложений. Копируешь текст → бабл пульсирует → нажимаешь → видишь перевод.

---

### Фаза 5: Accessibility Service — выделение текста  🟡 SHOULD

**Что**: опциональная фича. Пользователь включает вручную.

#### Задача 5.1 — Accessibility Service
- [ ] `service/TranslatorAccessibilityService.kt`
- [ ] Фильтр: `TYPE_VIEW_TEXT_SELECTION_CHANGED`
- [ ] Извлечение выделенного текста через `event.source.textSelectionStart/End`
- [ ] Отправка текста в OverlayService

#### Задача 5.2 — Кнопка «Перевести» у выделения
- [ ] Показать маленькую плашку рядом с выделенным текстом
- [ ] Нажатие → развернуть панель перевода с выделенным текстом
- [ ] Альтернатива: просто показать/пульсировать бабл (проще)

**Результат фазы 5**: выделил текст → кнопка «Перевести» → перевод.

---

### Фаза 6: Полировка и edge cases  🟢 COULD

#### Задача 6.1 — Обработка ошибок
- [ ] Нет интернета → показать сообщение, сохранить текст для повтора
- [ ] API ошибка (401/429/5xx) → понятное сообщение пользователю
- [ ] Пустой буфер → ничего не делать
- [ ] Текст > 5000 символов → обрезать с предупреждением

#### Задача 6.2 — Выбор языка вручную
- [ ] DropdownMenu в панели: выбор исходного и целевого языков
- [ ] Авто-детекция + ручной оверрайд
- [ ] Сохранение последнего выбранного языка

#### Задача 6.3 — Тёмная тема
- [ ] Адаптация под системную тему
- [ ] Material3 dynamic colors

#### Задача 6.4 — Тесты
- [ ] Unit: LanguageSelector (алгоритм)
- [ ] Unit: TranslateUseCase (mock API)
- [ ] Integration: ClipboardService → DataStore
- [ ] Integration: OverlayService → WindowManager (на эмуляторе)

**Результат фазы 6**: production-ready приложение.

---

## Граф зависимостей

```
Фаза 1 (скелет)
  │
  ├─► Фаза 2 (перевод)
  │     │
  │     └─► Фаза 3 (буфер обмена)
  │           │
  │           └─► Фаза 4 (плавающий бабл)
  │                 │
  │                 ├─► Фаза 5 (accessibility)
  │                 └─► Фаза 6 (полировка)
  │
  └─► (можно параллелить фазу 3 пока фаза 2 доделывается,
       но проще последовательно)
```

---

## Структура кода (целевая)

```
android/app/src/main/java/com/translator/android/
│
├── TranslatorApp.kt                   # Application class
├── MainActivity.kt                    # Точка входа → экран настроек
│
├── api/
│   ├── OpenRouterApi.kt              # Retrofit interface
│   ├── ApiClient.kt                  # Singleton Retrofit builder
│   └── dto/                          # Data Transfer Objects
│       ├── ChatRequest.kt
│       ├── Message.kt
│       ├── ChatResponse.kt
│       └── Choice.kt
│
├── data/
│   ├── SettingsDataStore.kt           # Preferences DataStore wrapper
│   └── model/
│       └── Language.kt               # Language enum
│
├── domain/
│   ├── TranslateUseCase.kt           # Бизнес-логика перевода
│   └── TranslationError.kt           # Свои типы ошибок
│
├── language/
│   ├── LanguageDetector.kt           # ML Kit обёртка
│   └── LanguageSelector.kt           # Алгоритм chooseTargetLanguage
│
├── service/
│   ├── ClipboardService.kt           # Foreground service: буфер обмена
│   ├── OverlayService.kt             # Foreground service: WindowManager
│   └── TranslatorAccessibilityService.kt  # Accessibility (v1.1)
│
└── ui/
    ├── theme/
    │   └── Theme.kt                  # Material3 theme
    │
    ├── settings/
    │   ├── SettingsScreen.kt         # Экран настроек
    │   └── SettingsViewModel.kt
    │
    ├── bubble/
    │   ├── BubbleView.kt             # Круглая плавающая иконка
    │   └── BubbleViewModel.kt
    │
    └── panel/
        ├── TranslationPanel.kt       # Развёрнутая панель перевода
        └── PanelViewModel.kt
```

---

## Оценка трудозатрат

| Фаза | Описание | Сложность | ~Часов |
|---|---|---|---|
| 1 | Скелет + конфигурация | Низкая | 3–4 |
| 2 | Перевод (API + логика) | Средняя | 5–8 |
| 3 | Буфер обмена | Средняя | 3–5 |
| 4 | Плавающий бабл | Высокая | 8–12 |
| 5 | Accessibility Service | Средняя | 3–5 |
| 6 | Полировка | Средняя | 4–6 |
| **Итого** | | | **26–40** |

---

## Что НЕ входит в v1

- История переводов
- Избранное (⭐)
- OCR (перевод текста на скриншотах)
- Офлайн-перевод (ML Kit Translation)
- Поддержка нескольких API-провайдеров
- Экспорт/импорт конфига
- Логирование (debug mode)

---

## Первый шаг

```bash
cd android/
make build    # Убедиться что скелет собирается
```

Затем — начать Фазу 1: создать `Language.kt`, `SettingsDataStore.kt`, переработать `MainActivity.kt`.

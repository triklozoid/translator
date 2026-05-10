# Translator Android — Функциональная спецификация

> Версия 1.0 | 2026-05-09

## 0. Концепция

Мобильный порт [Clipboard Translator](../README.md) — приложение-переводчик на базе LLM API (OpenRouter/OpenAI), адаптированное под сценарии мобильного использования.

Ключевое отличие от десктопной версии: **перманентный плавающий попап** (floating bubble), который висит поверх любых приложений и обеспечивает мгновенный доступ к переводу.

---

## 1. Основные возможности

### 1.1 Перевод текста
- Перевод через OpenRouter API (совместимое с OpenAI Chat Completions)
- Автоматическое определение языка источника (через ML Kit Language Identification или эвристики)
- Умный выбор языка перевода (по логике оригинала: primary/secondary/last language)
- Ручной выбор исходного и целевого языков

### 1.2 Плавающий попап (Floating Bubble)
Перманентный круглый значок, висящий поверх всех приложений. При нажатии раскрывается в панель перевода.

| Состояние | Описание |
|---|---|
| Свёрнут (bubble) | Маленький круглый значок (≈56dp), полупрозрачный, можно перетаскивать по краям экрана |
| Развёрнут (panel) | Компактная панель (≈60% высоты экрана) с полем текста, выбором языка, результатом перевода |
| Скрыт | Пользователь может временно скрыть бабл (через настройки или долгое нажатие) |

### 1.3 Способы захвата текста

| Метод | Как работает | Требует |
|---|---|---|
| **Clipboard listener** | При копировании текста в буфер обмена — бабл анимируется, текст автоматически подставляется в панель | Только foreground service (Android 10+) |
| **Accessibility Service** | При выделении текста в любом приложении — появляется кнопка «Перевести» рядом с выделением | Accessibility Service (пользователь включает вручную) |
| **Ручной ввод** | Открыть панель бабла и вставить/набрать текст руками | Ничего дополнительного |
| **Share intent** | Поделиться текстом из любого приложения → Translator | Стандартный intent-filter |

### 1.4 Работа с результатом
- **Копировать** — результат сразу в буфер обмена
- **Share** — поделиться переводом в другое приложение
- **Закрыть** — свернуть панель обратно в бабл
- **Изменить язык** — перевести на другой язык без повторного ввода

### 1.5 Конфигурация
- API ключ (OpenRouter)
- Primary / Secondary языки
- Список доступных языков
- Поведение бабла: авто-показ при копировании, позиция на экране, прозрачность

---

## 2. Пользовательский сценарий (User Flow)

```
Пользователь читает статью в Chrome на английском
        │
        ▼
Выделяет незнакомое слово → копирует в буфер
        │
        ▼
Бабл Translator коротко анимируется (пульсация)
        │
        ▼
Пользователь нажимает на бабл → раскрывается панель
        │
        ▼
Текст уже вставлен, язык определён автоматически (EN→RU)
        │
        ▼
Пользователь видит перевод, нажимает «Копировать и закрыть»
        │
        ▼
Перевод в буфере, панель свернулась → можно вставить в заметки
```

---

## 3. Техническая архитектура

### 3.1 Компоненты

```
┌──────────────────────────────────────────────┐
│                  Android OS                  │
│  ┌──────────┐  ┌──────────┐  ┌────────────┐ │
│  │Clipboard │  │Accessibil│  │ Share      │ │
│  │Manager   │  │ityService│  │ Intent     │ │
│  └────┬─────┘  └────┬─────┘  └─────┬──────┘ │
│       │             │              │         │
│  ┌────▼─────────────▼──────────────▼──────┐  │
│  │        Foreground Service             │  │
│  │  ┌─────────────────────────────────┐  │  │
│  │  │  WindowManager (Floating View)  │  │  │
│  │  │  ┌──────────┐  ┌─────────────┐  │  │  │
│  │  │  │ Bubble   │  │ Translation │  │  │  │
│  │  │  │ View     │◄─┤ Panel       │  │  │  │
│  │  │  └──────────┘  └──────┬──────┘  │  │  │
│  │  └───────────────────────┼─────────┘  │  │
│  │                          │            │  │
│  │  ┌───────────────────────▼─────────┐  │  │
│  │  │        ViewModel               │  │  │
│  │  │  - TranslateUseCase            │  │  │
│  │  │  - LanguageDetectUseCase       │  │  │
│  │  │  - ClipboardMonitor            │  │  │
│  │  └───────────────┬────────────────┘  │  │
│  │                  │                   │  │
│  │  ┌───────────────▼────────────────┐  │  │
│  │  │        Repository             │  │  │
│  │  │  - OpenRouterApi (Retrofit)   │  │  │
│  │  │  - LanguageDetector (ML Kit)  │  │  │
│  │  │  - SettingsStore (DataStore)  │  │  │
│  │  └───────────────────────────────┘  │  │
│  └─────────────────────────────────────┘  │
└──────────────────────────────────────────────┘
```

### 3.2 Точки входа в приложение

1. **MainActivity** — главный экран настроек (языки, API ключ, тумблеры)
2. **OverlayService** — foreground service, управляет WindowManager и плавающим баблом
3. **TranslatorAccessibilityService** — опциональный accessibility service для захвата выделенного текста

### 3.3 Жизненный цикл бабла

```
App запущен
  │
  ├─► MainActivity: настройка языков / API ключа
  │
  ├─► Пользователь включает «Показывать бабл»
  │     │
  │     ▼
  │   OverlayService.startForeground()
  │   WindowManager.addView(bubbleView)
  │     │
  │     ├─► ClipboardListener: текст скопирован → бабл пульсирует
  │     │
  │     ├─► Тап по баблу → развернуть TranslationPanel
  │     │     │
  │     │     ├─► Авто-определение языка
  │     │     ├─► Запрос к OpenRouter API
  │     │     ├─► Показ результата
  │     │     └─► Копировать / Share / Свернуть
  │     │
  │     └─► Свайп бабла в «корзину» (низ экрана) → скрыть
  │
  └─► Пользователь выключает «Показывать бабл»
        │
        ▼
      OverlayService.stopSelf()
      WindowManager.removeView(bubbleView)
```

---

## 4. Необходимые разрешения (Permissions)

| Permission | Зачем | Как запрашивается |
|---|---|---|
| `INTERNET` | Запросы к OpenRouter API | Автоматически (normal permission) |
| `SYSTEM_ALERT_WINDOW` | Показ бабла поверх других приложений | Intent → Settings → «Поверх других приложений» |
| `FOREGROUND_SERVICE` + `FOREGROUND_SERVICE_SPECIAL_USE` | Foreground service для WindowManager (Android 14+) | Автоматически |
| `POST_NOTIFICATIONS` | Нотификация foreground service (Android 13+) | Диалог при первом запуске |
| `BIND_ACCESSIBILITY_SERVICE` | Опционально — захват выделенного текста | Пользователь вручную: Settings → Accessibility |

---

## 5. Реализация — ключевые моменты

### 5.1 Floating Bubble (WindowManager)

Базовая схема:

```kotlin
// В OverlayService (Foreground Service)
class OverlayService : Service() {

    private lateinit var windowManager: WindowManager
    private lateinit var bubbleView: View

    override fun onCreate() {
        super.onCreate()
        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager

        // Создаём бабл через ComposeView
        bubbleView = ComposeView(this).apply {
            setContent {
                BubbleContent(
                    onClick = { expandPanel() },
                    text = currentClipboardText
                )
            }
        }

        val params = WindowManager.LayoutParams(
            WRAP_CONTENT, WRAP_CONTENT,
            TYPE_APPLICATION_OVERLAY,             // Поверх всех приложений
            FLAG_NOT_FOCUSABLE or FLAG_NOT_TOUCH_MODAL,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.TOP or Gravity.START
            x = 100; y = 300
        }

        windowManager.addView(bubbleView, params)

        // Перетаскивание бабла
        bubbleView.setOnTouchListener(BubbleTouchListener(windowManager, params))
    }
}
```

**Важно для Android 10+**:  
- Foreground service обязателен для длительной работы WindowManager  
- `FOREGROUND_SERVICE_SPECIAL_USE` на Android 14+  
- Нотификация foreground service показывается в шторке

### 5.2 Clipboard Listener

```kotlin
private val clipboardListener = ClipboardManager.OnPrimaryClipChangedListener {
    val text = clipboard?.primaryClip?.getItemAt(0)?.text?.toString() ?: return@...
    if (text.isNotBlank() && text.length <= MAX_TRANSLATE_LENGTH) {
        currentClipboardText = text
        // Анимировать бабл (пульсация)
        bubbleAnimatePulse()
        // Если панель развёрнута — сразу перевести
        if (isPanelExpanded) translate(text)
    }
}
```

**Ограничение**: на Android 10+ clipboard listener работает только пока foreground service активен.

### 5.3 Accessibility Service (опционально)

```kotlin
class TranslatorAccessibilityService : AccessibilityService() {

    override fun onServiceConnected() {
        val info = AccessibilityServiceInfo().apply {
            eventTypes = AccessibilityEvent.TYPE_VIEW_TEXT_SELECTION_CHANGED
            feedbackType = AccessibilityServiceInfo.FEEDBACK_GENERIC
            notificationTimeout = 100
        }
        setServiceInfo(info)
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent) {
        if (event.eventType != AccessibilityEvent.TYPE_VIEW_TEXT_SELECTION_CHANGED) return

        val source = event.source ?: return
        val selected = source.text.substring(
            source.textSelectionStart,
            source.textSelectionEnd
        )

        if (selected.isNotBlank()) {
            // Показать маленькую кнопку «Перевести» рядом с выделением
            // или отправить текст в OverlayService
        }
    }
}
```

### 5.4 OpenRouter API интеграция

Совместимо с OpenAI Chat Completions API. Модель по умолчанию: `openai/gpt-4o`.

```kotlin
// POST https://openrouter.ai/api/v1/chat/completions
// Headers:
//   Authorization: Bearer $OPENROUTER_API_KEY
//   HTTP-Referer: https://translator.app
//
// Body:
{
  "model": "openai/gpt-4o",
  "messages": [
    {
      "role": "system",
      "content": "Translate the following text to Russian. Return ONLY the translation, no explanations."
    },
    {
      "role": "user",
      "content": "The quick brown fox..."
    }
  ]
}
```

---

## 6. Экраны и UI

### 6.1 MainActivity (настройки)

```
┌─────────────────────────────────┐
│  Translator                     │
│                                 │
│  ── API ─────────────────────── │
│  API Key    [••••••••••sk-xxxx] │
│  Model      [openai/gpt-4o   ▼]│
│                                 │
│  ── Языки ────────────────────  │
│  Primary       [English     ▼]  │
│  Secondary     [Russian     ▼]  │
│  Available     EN, RU, FR, IT   │
│                                 │
│  ── Бабл ────────────────────   │
│  [✓] Показывать бабл            │
│  [✓] Авто-перевод при копировании│
│  [ ] Accessibility (выделение)  │
│                                 │
└─────────────────────────────────┘
```

### 6.2 Floating Panel (развёрнутый бабл)

```
┌─────────────────────────────────┐
│  ← Свернуть        ••• Меню     │
│                                 │
│  [EN ▼]  →  [RU ▼]    ⭐       │
│                                 │
│  ┌─────────────────────────────┐│
│  │ The quick brown fox jumps   ││
│  │ over the lazy dog           ││
│  └─────────────────────────────┘│
│                                 │
│  ┌─────────────────────────────┐│
│  │ Быстрая коричневая лиса     ││
│  │ перепрыгивает через         ││
│  │ ленивую собаку              ││
│  └─────────────────────────────┘│
│                                 │
│  [📋 Копировать]  [📤 Share]   │
└─────────────────────────────────┘
```

### 6.3 Bubble (свёрнутый)

```
     ┌──┐
     │🌐│  ← полупрозрачный, перетаскивается
     └──┘
```

---

## 7. Ограничения и edge cases

| Ситуация | Поведение |
|---|---|
| Нет интернета | Показать ошибку «Нет соединения», сохранить текст для повтора |
| API ключ не задан | При первом запуске показать экран настроек |
| Текст > 5000 символов | Обрезать с предупреждением |
| Буфер содержит нетекстовые данные | Игнорировать |
| Бабл перекрывает важный UI | Пользователь перетаскивает / временно скрывает |
| Убийство процесса системой | Foreground service + нотификация защищают от убийства; если убили — перезапуск через WorkManager |
| Android 14+ ограничения FGS | `foregroundServiceType="specialUse"` + проперти в манифесте |

---

## 8. Сравнение с десктопной версией

| Фича | Десктоп (Rust/GTK) | Android |
|---|---|---|
| Определение языка | lingua-rs (офлайн) | ML Kit Language ID или lingua-rs через JNI |
| API | OpenRouter | OpenRouter |
| Буфер обмена | GTK clipboard | ClipboardManager |
| UI | GTK окно | Floating bubble + Compose panel |
| Запуск | Ручной (`./run`) | Всегда в фоне (foreground service) |
| Конфиг | `~/.config/translator/config.toml` | DataStore / SharedPreferences |

---

## 9. Roadmap

### v1.0 — MVP
- [x] Настройка API ключа и языков
- [x] Перевод через OpenRouter
- [x] Плавающий бабл (WindowManager + Foreground Service)
- [x] Clipboard listener (авто-подстановка скопированного текста)
- [x] Копирование результата в буфер

### v1.1
- [ ] Accessibility Service: перевод выделенного текста
- [ ] История переводов (локально)
- [ ] Избранное (⭐)

### v1.2
- [ ] OCR: перевод текста на скриншотах
- [ ] Офлайн-перевод (через ML Kit Translation)
- [ ] Несколько API-провайдеров (OpenAI, Anthropic, локальный)

---

## 10. Полезные ссылки

- **WindowManager / Floating windows**: https://developer.android.com/reference/android/view/WindowManager
- **Foreground Service**: https://developer.android.com/develop/background-work/services/fgs
- **Accessibility Service**: https://developer.android.com/guide/topics/ui/accessibility/service
- **Clipboard Manager**: https://developer.android.com/reference/android/content/ClipboardManager
- **Notification Bubbles API**: https://developer.android.com/develop/ui/views/notifications/bubbles
- **OpenRouter API**: https://openrouter.ai/docs
- **ML Kit Language ID**: https://developers.google.com/ml-kit/language/identification
- **Пример floating bubble (Android)**: https://github.com/dofire/Floating-Bubble-View

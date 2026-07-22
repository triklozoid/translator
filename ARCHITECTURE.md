# Architecture

## Overview

Clipboard Translator is a desktop application built with Rust and GTK4. It reads text from the system clipboard, detects the source language, and translates it using an LLM API (OpenRouter, OpenAI, Moonshot, Ollama, DeepSeek, or any OpenAI-compatible endpoint). The translation result appears incrementally via streaming.

```
┌─────────────────────────────────────────────────────────┐
│                        User                              │
│  (selects text → Ctrl+C → launches app → clicks lang)   │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│                     main.rs                              │
│  - CLI parsing (clap)                                     │
│  - Single-instance check                                  │
│  - tokio runtime bootstrap                                │
│  - GTK Application init                                   │
│  - delegates to ui::build_ui()                            │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│                      ui.rs                               │
│  ┌──────────────┐  ┌───────────────┐  ┌───────────────┐ │
│  │ Clipboard     │  │ Language       │  │ Translation   │ │
│  │ read          │──│ detection      │──│ orchestration │ │
│  │ (GDK)         │  │ (lingua crate) │  │ (calls trans- │ │
│  │               │  │                │  │ lation module)│ │
│  └──────────────┘  └───────────────┘  └───────┬───────┘ │
│                                                │         │
│  ┌──────────────────────────────────────────────┘       │
│  │ UI updates (label, status bar, button toggles)        │
│  │ via glib::spawn_future_local                          │
│  └──────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
                  │ calls
                  ▼
┌─────────────────────────────────────────────────────────┐
│                  translation.rs                          │
│  - translate_text(): blocking (non-streaming) version    │
│  - translate_text_streaming(): incremental via mpsc      │
│  - request_translation(): UI glue — spawns streaming    │
│    task + listens for chunks + updates GTK labels        │
│  - retry logic: 3 attempts, exponential backoff (1s→2s→4s)│
│  - cancellation: AtomicBool set by oneshot channel       │
│  - provider auto-detection: resolve_api_key() in ui.rs   │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│              llm-connector crate (lib/)                  │
│  - LlmClient (moonshot_with_config, 120s timeout)        │
│  - ChatRequest / Message types                           │
│  - chat() and chat_stream()                              │
└─────────────────────────────────────────────────────────┘
```

## Module Map

| Module | File | Responsibility |
|--------|------|---------------|
| `main` | `src/main.rs` | Entry point, CLI, single-instance, tokio + GTK bootstrap |
| `lib` | `src/lib.rs` | Crate root for library (re-exports, `clone!` macro) |
| `ui` | `src/ui.rs` | All GTK4 widgets, clipboard read, language detection, translation orchestration, button/state management (`~31 KB`, largest module) |
| `translation` | `src/translation.rs` | LLM API calls: streaming, non-streaming, retry, cancellation, error humanization, UI glue (`request_translation`) |
| `config` | `src/config.rs` | `Config` struct, serde for `lingua::Language`, load/save `~/.config/translator/config.toml` with backup of invalid files |
| `settings` | `src/settings.rs` | Persist `last_language.txt` (ISO 639-1 code) to `~/.config/translator/` |
| `clipboard_utils` | `src/clipboard_utils.rs` | `read_clipboard_text()` helper (mostly unused — UI reads clipboard directly via GDK) |
| `logger` | `src/logger.rs` | Writes translation prompts and responses to `~/.config/translator/app.log` when `debug: true` |
| `language` | `src/language.rs` | **Deprecated stub** — kept for backward compatibility, no actual code |
| `bin/translator-gui` | `src/bin/` | Secondary binary target (Tauri/web-based GUI, experimental) |

## Data Flow

### Startup Sequence

```
main()
  ├─ Args::parse()              → CLI flags (--debug)
  ├─ SingleInstance::new()      → prevents duplicate instances
  ├─ dotenv()                   → loads .env file for API keys
  ├─ config::load_config()      → reads config.toml (or creates default)
  │   └─ if --debug: config.debug = true
  ├─ Application::builder()     → GTK4 app with HANDLES_COMMAND_LINE
  └─ connect_activate()         → ui::build_ui(app, config)
```

### Initial Translation Flow

```
build_ui()
  ├─ config_rc: Rc<RefCell<Config>>
  ├─ logger_rc: Rc<Logger>
  ├─ detector: LanguageDetector (lingua, low accuracy, primary lang only)
  │
  └─ glib::spawn_future_local(async {
       1. resolve_api_key(config.api_url)
          → detects provider from URL → reads env var → stores key
       2. clipboard.read_text_future()
          → GString → String → stores in original_clipboard_text
       3. detector.detect_language_of(sample[0..100])
          → timeout 2s via tokio::time::timeout
       4. choose_target_language(detected, primary, secondary, last)
          → algorithm from README
       5. settings::save_last_language(target)
          → writes ISO code to last_language.txt
       6. update_active_button_simple(target)
          → sets ToggleButton state on UI thread via idle_add_local_once
       7. request_translation(text, target, ...)
          → spawns translation task → streams chunks → updates label
     })
```

### User Clicks Language Button

```
ToggleButton::connect_toggled(handler)
  ├─ if !is_active() && lang == current_target:
  │   → re-activate button (prevent deselecting the only active button)
  │
  ├─ if is_active() && lang != current_target:
  │   ├─ current_target.set(lang)
  │   ├─ settings::save_last_language(lang)
  │   ├─ suppress_toggles = true → deactivate other buttons → suppress_toggles = false
  │   ├─ cancel_sender.send(())  → cancel ongoing translation
  │   ├─ generation += 1
  │   └─ request_translation(original_text, lang, ...)
  │       → streaming translation starts
  │
  └─ if is_active() && lang == current_target:
      → ensure other buttons are deactivated (consistency)
```

### Translation Internals

```
request_translation(text, target, ..., label, status_label, cancel_rx, generation)
  ├─ label.set_text("Translating to X...")
  ├─ status_label.set_text("Translating...")
  │
  ├─ mpsc::unbounded_channel()   → chunk_tx / chunk_rx
  ├─ oneshot::channel()          → result_tx / result_rx
  │
  ├─ glib::spawn_future_local(translate_text_streaming(...))
  │   ├─ build_client() → LlmClient::moonshot_with_config(120s timeout)
  │   ├─ ChatRequest with reasoning_effort="none", max_tokens=4096
  │   ├─ chat_stream() with retry loop (up to 3 attempts)
  │   ├─ cancellation: AtomicBool polled between chunks + during backoff sleep
  │   └─ sends chunks via chunk_tx, completion signal via result_tx
  │
  └─ loop { select! {
       chunk_rx.recv() → accumulated += chunk → label.set_text(accumulated)
       result_rx       → status_label.set_text("completed/failed in X.Xs")
     }}
       └─ generation check: only update label if generation == current_generation
          (prevents stale updates from cancelled translations)
```

### Translation Retry Logic

```
for attempt in 1..=3:
  └─ client.chat_stream(&request).await
     ├─ Ok → break (success)
     ├─ Err(e) if e.is_retryable() && attempt < 3:
     │   → delay = min(1000 * 2^(attempt-1), 8000) ms
     │   → sleep with 100ms cancellation polling
     │   → continue
     └─ Err(e) → return humanize_error(e)
```

## State Management

All shared state uses `Rc<RefCell<T>>` (single-threaded, GTK main loop):

| Variable | Type | Purpose |
|----------|------|---------|
| `config_rc` | `Rc<RefCell<Config>>` | Config (API URL, model, languages, debug) |
| `logger_rc` | `Rc<Logger>` | Debug logger (cloneable) |
| `original_clipboard_text` | `Rc<RefCell<Option<String>>>` | Original text for re-translation |
| `api_key_rc` | `Rc<RefCell<Option<String>>>` | Resolved API key |
| `provider_name_rc` | `Rc<RefCell<String>>` | Detected provider name |
| `cancel_sender_rc` | `Rc<RefCell<Option<Sender<()>>>>` | Cancellation channel for current translation |
| `translation_generation` | `Rc<Cell<u64>>` | Monotonic counter — ensures stale translation results don't update UI |
| `current_target` | `Rc<Cell<Language>>` | Currently selected target language (in-memory source of truth) |
| `suppress_toggles` | `Rc<Cell<bool>>` | Prevents infinite toggle loops during programmatic button state changes |
| `language_buttons_rc` | `Rc<RefCell<Vec<(Language, Rc<RefCell<ToggleButton>>)>>>` | All language buttons |

## Configuration Files

```
~/.config/translator/
├── config.toml           # Main configuration (TOML)
├── last_language.txt     # ISO 639-1 code of last selected target language
└── app.log               # Translation debug log (only when debug: true)
```

Config schema (`config.toml`):

```toml
api_url = "https://openrouter.ai/api/v1"
model_version = "openai/gpt-4o"
primary_language = "EN"
secondary_language = "FR"
all_target_languages = ["EN", "FR", "IT", "PL"]
debug = false
```

## Dependencies

### Runtime

| Crate | Usage |
|-------|-------|
| `gtk4` v0.9.6 | GUI toolkit (GTK4 with v4_14 feature) |
| `lingua` v1.7 | Language detection (EN, FR, IT, PL, RU, PT, UK, DE, ES features) |
| `llm-connector` v0.5 | LLM API client (patched, vendored in `lib/llm-connector`) |
| `tokio` v1 | Async runtime for network I/O |
| `futures-util` | Stream combinators for streaming responses |
| `serde` + `toml` | Configuration serialization |
| `dirs` v5.0 | XDG config directory resolution |
| `dotenvy` v0.15 | `.env` file loading |
| `single-instance` v0.3 | Mutex-based single instance lock |
| `clap` v4.5 | CLI argument parsing (`--debug`) |
| `chrono` v0.4 | Timestamps for debug log entries |

### Dev

| Crate | Usage |
|-------|-------|
| `tempfile` | Temporary files in tests |
| `tokio-test` | Async test helpers |
| `wiremock` | HTTP mocking for API tests |

## Key Design Decisions

1. **`moonshot_with_config` for all providers**: `llm-connector`'s `openai_compatible()` silently ignores the builder timeout. Using `moonshot_with_config` ensures the 120s timeout propagates to the HTTP layer — needed for reasoning models.

2. **120s timeout**: Reasoning models (kimi-k2.6 etc.) can spend significant time on internal thinking before producing output.

3. **`reasoning_effort: "none"`**: Translation is a straightforward task; disabling reasoning reduces token usage and latency.

4. **`max_tokens: 4096`**: Generous buffer — actual translation output is typically <200 tokens, but long texts need headroom.

5. **No system messages**: Some providers (Ollama kimi-k2.6) mishandle system messages by echoing them back. Instructions are merged into the user message.

6. **Mid-stream errors not retried**: Retry only applies when establishing the stream (5xx on initial connect). Once streaming begins, errors surface directly to UI.

7. **Generation counter for cancellation**: A monotonic `u64` prevents stale chunks from cancelled translations from overwriting the new translation's output. Every `request_translation` call increments the counter and checks it before updating the label.

8. **Single-instance via file lock**: `single_instance` crate places a lockfile — simpler than D-Bus activation and sufficient for a single-user desktop tool.

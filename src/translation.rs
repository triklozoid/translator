use gtk::prelude::*;
use gtk::{Label};
use lingua::Language;
use llm_connector::LlmClient;
use llm_connector::types::{ChatRequest, Message};
use futures_util::StreamExt;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::oneshot;
use crate::diag_log;
use crate::logger::Logger;

// Result type for translations
pub type TranslationResult = Result<String, String>;

/// Maximum retry attempts for transient errors (5xx, network, rate-limit, timeout).
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (milliseconds).
const BASE_RETRY_DELAY_MS: u64 = 1000;   // 1s

/// Maximum delay cap (milliseconds).
const MAX_RETRY_DELAY_MS: u64 = 8000;    // 8s

/// Build an LlmClient for the given provider, key, and URL.
/// Uses a generous 120s timeout because reasoning models (kimi-k2.6, etc.)
/// can spend significant time on internal thinking before responding.
///
/// NOTE: We use `moonshot_with_config` for ALL providers because
/// `openai_compatible()` in llm-connector 0.5.19 silently ignores the builder
/// timeout — it always creates an HttpClient with the default 60s timeout.
/// `moonshot_with_config` properly passes timeout_secs to the HTTP layer.
fn build_client(
    provider_name: &str,
    api_key: &str,
    api_url: &str,
) -> Result<LlmClient, String> {
    LlmClient::moonshot_with_config(api_key, Some(api_url), Some(120), None)
        .map_err(|e| format!("Failed to create client for {}: {}", provider_name, e))
}

/// Map llm_connector errors to user-friendly messages.
fn humanize_error(e: llm_connector::LlmConnectorError) -> String {
    use llm_connector::LlmConnectorError::*;

    match e {
        AuthenticationError(msg) => format!(
            "Authentication error (401). Check API key and provider.\n\
             Hint: the app auto-detects which env var to use based on api_url.\n\
             - api.moonshot.ai → MOONSHOT_API_KEY\n\
             - ollama.com → OLLAMA_API_KEY\n\
             - api.deepseek.com → DEEPSEEK_API_KEY\n\
             - api.openai.com → OPENAI_API_KEY\n\
             - Other → OPENROUTER_API_KEY\n\
             Details: {}",
            msg
        ),
        RateLimitError(msg) => format!("Rate limit reached (429). Please retry later. Details: {}", msg),
        InvalidRequest(msg) => format!("Invalid request (400). Details: {}", msg),
        UnsupportedModel(msg) => format!("Unsupported model. Details: {}", msg),
        ContextLengthExceeded(msg) => format!("Input text too long for this model. Details: {}", msg),
        PermissionError(msg) => format!("Access denied (403). Details: {}", msg),
        ServerError(msg) => format!("Server error (5xx). The provider may be experiencing issues. Details: {}", msg),
        TimeoutError(msg) => format!("Request timed out. Details: {}", msg),
        NetworkError(msg) | ConnectionError(msg) => format!("Network error. Check your internet connection. Details: {}", msg),
        other => format!("API Error: {}", other),
    }
}

/// Compute retry delay with exponential backoff, capped at MAX_RETRY_DELAY_MS.
fn compute_retry_delay(attempt: u32) -> Duration {
    let base = BASE_RETRY_DELAY_MS * 2u64.pow(attempt.saturating_sub(1));
    let capped = base.min(MAX_RETRY_DELAY_MS);
    Duration::from_millis(capped)
}

// Core translation function without UI dependencies with cancellation support
pub async fn translate_text(
    text_to_translate: &str,
    target_language: Language,
    api_key: String,
    api_url: String,
    model_version: String,
    provider_name: &str,
    cancel_receiver: Option<oneshot::Receiver<()>>,
    logger: Option<&Logger>,
) -> TranslationResult {
    // Check if text is empty before making API call
    if text_to_translate.trim().is_empty() {
        return Err("Clipboard text is empty.".to_string());
    }

    // Build the client
    let client = build_client(provider_name, &api_key, &api_url)?;

    // Prepare the prompt.
    // NOTE: kimi-k2.6 on Ollama mishandles system messages — it may echo
    // the system content back. We merge system instructions into the user
    // message instead.
    let target_language_name = if target_language == Language::Portuguese {
        "European Portuguese"
    } else {
        &target_language.to_string()
    };
    let combined_prompt = format!(
        "Translate the following text into {}. Provide only the translation, nothing else.\n\n{}",
        target_language_name,
        text_to_translate,
    );

    // Build chat request — reasoning is disabled for fast translation.
    // max_tokens=4096 is generous but actual translation output is typically <200 tokens.
    let request = ChatRequest::new(&model_version)
        .with_max_tokens(4096)
        .with_reasoning_effort("none")
        .add_message(Message::user(&combined_prompt));

    let log_prompt = combined_prompt.clone();
    let log_target = target_language.to_string();

    // Convert oneshot cancellation to a reusable AtomicBool flag.
    // oneshot is consumed on first .await, so we spawn a task that
    // sets the flag when cancellation fires.
    let cancel_flag = Arc::new(AtomicBool::new(false));
    if let Some(cancel_rx) = cancel_receiver {
        let flag = Arc::clone(&cancel_flag);
        tokio::spawn(async move {
            let _ = cancel_rx.await;
            flag.store(true, Ordering::SeqCst);
        });
    }

    // Execute with retry loop for transient errors (5xx, network, rate-limit, timeout).
    // Exponential backoff: 1s → 2s → 4s, capped at 8s.
    let mut chat_response = None;
    for attempt in 1..=MAX_RETRIES {
        if cancel_flag.load(Ordering::SeqCst) {
            return Err("Translation cancelled".to_string());
        }

        match client.chat(&request).await {
            Ok(resp) => {
                chat_response = Some(resp);
                break;
            }
            Err(e) if e.is_retryable() && attempt < MAX_RETRIES => {
                let delay = compute_retry_delay(attempt);
                eprintln!(
                    "[Translation] retryable error (attempt {}/{}): {}. Retrying in {:?}...",
                    attempt, MAX_RETRIES, e, delay
                );
                // Poll cancellation every 100ms during backoff sleep
                let start = std::time::Instant::now();
                while start.elapsed() < delay {
                    if cancel_flag.load(Ordering::SeqCst) {
                        return Err("Translation cancelled".to_string());
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                continue;
            }
            Err(e) => {
                return Err(humanize_error(e));
            }
        }
    }

    // Safe: the loop either returns early with Err, or chat_response is Some.
    let response = match chat_response {
        Some(resp) => resp,
        None => {
            return Err(humanize_error(
                llm_connector::LlmConnectorError::MaxRetriesExceeded(
                    format!("All {} retry attempts failed", MAX_RETRIES)
                )
            ));
        }
    };

    // --- Detailed diagnostics ---
    let usage = response.usage.as_ref();
    eprintln!(
        "[Translation] model={} tokens: prompt={} completion={} total={} finish={:?}",
        response.model,
        usage.map(|u| u.prompt_tokens).unwrap_or(0),
        usage.map(|u| u.completion_tokens).unwrap_or(0),
        usage.map(|u| u.total_tokens).unwrap_or(0),
        response.choices.first().and_then(|c| c.finish_reason.as_deref()),
    );

    // Extract translation text.
    // Priority: 1) message.content (normal output),
    //           2) reasoning fields (for reasoning models with thinking enabled),
    //           3) convenience content field (fallback).
    let mut response_text: String = response
        .choices
        .first()
        .map(|c| c.message.content_as_text().trim().to_string())
        .unwrap_or_default();

    if response_text.is_empty() {
        // Try to extract from reasoning/thinking fields
        if let Some(choice) = response.choices.first() {
            let reasoning_fallback = choice.message.reasoning_content.as_deref()
                .or(choice.message.reasoning.as_deref())
                .or(choice.message.thinking.as_deref())
                .unwrap_or("");
            if !reasoning_fallback.is_empty() {
                eprintln!(
                    "[Translation] content was empty, using reasoning fallback ({} chars).\n\
                     Hint: reasoning models spend tokens on thinking. If translation is\n\
                     incomplete, increase max_tokens or disable thinking mode.",
                    reasoning_fallback.len()
                );
                response_text = reasoning_fallback.trim().to_string();
            }
        }
    }

    // Final fallback: convenience content field
    if response_text.is_empty() {
        response_text = response.content.trim().to_string();
    }

    if response_text.is_empty() {
        let finish = response.choices.first()
            .and_then(|c| c.finish_reason.as_deref())
            .unwrap_or("unknown");
        return Err(format!(
            "API returned empty translation (finish_reason={}, {} completion tokens).\n\
             If using a reasoning model, try increasing max_tokens.",
            finish,
            usage.map(|u| u.completion_tokens).unwrap_or(0),
        ));
    }

    eprintln!("[Translation] response: {} chars", response_text.len());

    // Log the translation if logger is available
    if let Some(logger) = logger {
        logger.log_translation(&log_prompt, &response_text, &log_target);
    }

    Ok(response_text)
}

/// Maximum idle time between stream chunks before we consider the connection stalled.
const STREAM_STALL_TIMEOUT_SECS: u64 = 60;

/// Streaming version of translate_text — directly updates the label with
/// incremental chunks as they arrive from the LLM.
///
/// Runs on the GLib main context (called via glib::spawn_future_local),
/// so it can safely call label.set_text() without any channel.
pub async fn translate_text_streaming(
    text_to_translate: &str,
    target_language: Language,
    api_key: String,
    api_url: String,
    model_version: String,
    provider_name: &str,
    cancel_receiver: Option<oneshot::Receiver<()>>,
    logger: Option<&Logger>,
    label: &Label,
    generation: u64,
    current_generation: Rc<Cell<u64>>,
) -> Result<String, String> {
    if text_to_translate.trim().is_empty() {
        return Err("Clipboard text is empty.".to_string());
    }

    let client = build_client(provider_name, &api_key, &api_url)?;

    let target_language_name = if target_language == Language::Portuguese {
        "European Portuguese"
    } else {
        &target_language.to_string()
    };
    let combined_prompt = format!(
        "Translate the following text into {}. Provide only the translation, nothing else.\n\n{}",
        target_language_name,
        text_to_translate,
    );

    let request = ChatRequest::new(&model_version)
        .with_max_tokens(4096)
        .with_reasoning_effort("none")
        .add_message(Message::user(&combined_prompt));

    let log_prompt = combined_prompt.clone();
    let log_target = target_language.to_string();

    let mut full_text = String::new();

    // Convert oneshot cancellation to a reusable AtomicBool flag.
    let cancel_flag = Arc::new(AtomicBool::new(false));
    if let Some(cancel_rx) = cancel_receiver {
        let flag = Arc::clone(&cancel_flag);
        tokio::spawn(async move {
            let _ = cancel_rx.await;
            flag.store(true, Ordering::SeqCst);
        });
    }

    // Retry loop for transient errors when establishing the stream.
    // Once the stream is established and yielding data, mid-stream errors
    // are NOT retried (they surface to the UI as before).
    let mut stream_opt = None;
    for attempt in 1..=MAX_RETRIES {
        if cancel_flag.load(Ordering::SeqCst) {
            return Err("Translation cancelled".to_string());
        }

        match client.chat_stream(&request).await {
            Ok(s) => {
                stream_opt = Some(s);
                break;
            }
            Err(e) if e.is_retryable() && attempt < MAX_RETRIES => {
                let delay = compute_retry_delay(attempt);
                eprintln!(
                    "[Translation] streaming retry (attempt {}/{}): {}. Retrying in {:?}...",
                    attempt, MAX_RETRIES, e, delay
                );
                // Poll cancellation every 100ms during backoff sleep
                let start = std::time::Instant::now();
                while start.elapsed() < delay {
                    if cancel_flag.load(Ordering::SeqCst) {
                        return Err("Translation cancelled".to_string());
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                continue;
            }
            Err(e) => {
                return Err(humanize_error(e));
            }
        }
    }

    // Safe: the loop either returned early with Err, or stream_opt is Some.
    let mut stream = match stream_opt {
        Some(s) => s,
        None => {
            return Err(humanize_error(
                llm_connector::LlmConnectorError::MaxRetriesExceeded(
                    format!("All {} retry attempts failed", MAX_RETRIES)
                )
            ));
        }
    };

    // Consume stream with cancellation checks and stall timeout between chunks.
    // Uses tokio::time::timeout on each stream.next() to detect stalls.
    let mut chunk_idx: u64 = 0;
    let mut skipped_count: u64 = 0;
    let stall_timeout = Duration::from_secs(STREAM_STALL_TIMEOUT_SECS);
    let stream_start = std::time::Instant::now();
    let mut last_chunk_at = stream_start;

    diag_log!("[Stream] consuming SSE stream (stall_timeout={}s)...", STREAM_STALL_TIMEOUT_SECS);

    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            diag_log!("[Stream] cancelled at chunk #{} ({}s elapsed, {} chars accumulated)",
                chunk_idx, stream_start.elapsed().as_secs_f64(), full_text.len());
            return Err("Translation cancelled".to_string());
        }

        match tokio::time::timeout(stall_timeout, stream.next()).await {
            Ok(Some(chunk)) => {
                chunk_idx += 1;
                last_chunk_at = std::time::Instant::now();
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        let msg = humanize_error(e);
                        diag_log!("[Stream] error at chunk #{} ({}s elapsed): {}",
                            chunk_idx, stream_start.elapsed().as_secs_f64(), msg);
                        return Err(msg);
                    }
                };
                if let Some(content) = chunk.get_content() {
                    full_text.push_str(content);
                    diag_log!("[Stream] chunk #{:<3} +{}ms: {} chars (total={})",
                        chunk_idx,
                        last_chunk_at.duration_since(stream_start).as_millis(),
                        content.len(), full_text.len());
                    // Update label directly (we're on the GLib main context)
                    if generation == current_generation.get() {
                        label.set_text(&full_text);
                    }
                } else {
                    skipped_count += 1;
                    diag_log!("[Stream] chunk #{:<3} +{}ms: skipped (get_content returned None)",
                        chunk_idx,
                        last_chunk_at.duration_since(stream_start).as_millis());
                }
            }
            Ok(None) => {
                // Stream ended normally
                diag_log!("[Stream] ended normally: {} chunks total ({}s), {} with content ({} chars), {} skipped",
                    chunk_idx, stream_start.elapsed().as_secs_f64(),
                    chunk_idx - skipped_count, full_text.len(), skipped_count);
                break;
            }
            Err(_elapsed) => {
                // Stall timeout — no chunk received for STREAM_STALL_TIMEOUT_SECS
                let idle = last_chunk_at.elapsed();
                diag_log!(
                    "[Stream] STALL DETECTED: no chunk for {:.0}s (total elapsed={:.0}s, {} chunks, {} chars). Terminating stream.",
                    idle.as_secs_f64(), stream_start.elapsed().as_secs_f64(),
                    chunk_idx, full_text.len()
                );
                if full_text.is_empty() {
                    return Err(format!(
                        "Translation stream stalled after {:.0}s with no output. The LLM may be overloaded or the connection was dropped.",
                        stream_start.elapsed().as_secs_f64()
                    ));
                }
                // We have partial output — treat as truncated completion
                diag_log!("[Stream] returning partial translation ({} chars) after stall", full_text.len());
                break;
            }
        }
    }

    // Validate: ensure we got non-empty output
    let response_text = full_text.trim().to_string();
    if response_text.is_empty() {
        return Err(
            "API returned empty translation.\n\
             If using a reasoning model, try increasing max_tokens.".to_string(),
        );
    }

    diag_log!("[Translation] streaming response: {} chars", response_text.len());

    // Final label update to ensure complete text is shown
    if generation == current_generation.get() {
        label.set_text(&response_text);
    }

    if let Some(logger) = logger {
        logger.log_translation(&log_prompt, &response_text, &log_target);
    }

    Ok(response_text)
}

// --- Helper function to request translation ---
// Simple wrapper: calls translate_text_streaming which directly updates
// the label on each chunk (it runs on the GLib main context).
pub async fn request_translation(
    text_to_translate: String,
    target_language: Language,
    api_key: String,
    api_url: String,
    model_version: String,
    provider_name: String,
    label_to_update: Label,
    status_label: Label,
    cancel_receiver: Option<oneshot::Receiver<()>>,
    logger: Option<Logger>,
    generation: u64,
    current_generation: Rc<Cell<u64>>,
) {
    label_to_update.set_label(&format!("Translating to {}...", target_language));
    status_label.set_text("Translating...");

    diag_log!("[Stream] translation started: generation={}, target={:?}",
        generation, target_language);

    let start_time = std::time::Instant::now();
    match translate_text_streaming(
        &text_to_translate,
        target_language,
        api_key,
        api_url,
        model_version,
        &provider_name,
        cancel_receiver,
        logger.as_ref(),
        &label_to_update,
        generation,
        current_generation.clone(),
    )
    .await
    {
        Ok(_translated_text) => {
            let elapsed = start_time.elapsed();
            diag_log!("[Stream] translation finished in {:.1}s", elapsed.as_secs_f64());
            if generation == current_generation.get() {
                status_label.set_text(&format!("Translation completed in {:.1}s", elapsed.as_secs_f64()));
            }
        }
        Err(error_message) => {
            let elapsed = start_time.elapsed();
            diag_log!("[Stream] translation failed in {:.1}s: {}", elapsed.as_secs_f64(), error_message);
            if error_message != "Translation cancelled" {
                diag_log!("Translation Error: {}", error_message);
                if generation == current_generation.get() {
                    label_to_update.set_text(&error_message);
                }
            }
            status_label.set_text(&format!("Translation failed after {:.1}s", elapsed.as_secs_f64()));
        }
    }
}

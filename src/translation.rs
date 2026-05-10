use gtk::Label;
use lingua::Language;
use llm_connector::LlmClient;
use llm_connector::types::{ChatRequest, Message};
use std::cell::Cell;
use std::rc::Rc;
use tokio::sync::oneshot;
use crate::logger::Logger;

// Result type for translations
pub type TranslationResult = Result<String, String>;

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

    // Execute with optional cancellation
    let result = match cancel_receiver {
        Some(cancel_rx) => {
            tokio::select! {
                result = client.chat(&request) => result,
                _ = cancel_rx => return Err("Translation cancelled".to_string()),
            }
        }
        None => client.chat(&request).await,
    };

    match result {
        Ok(response) => {
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
        Err(e) => Err(humanize_error(e)),
    }
}

// --- Helper function to request translation ---
// UI wrapper around core translation function with cancellation support
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
    // Update UI to show translation in progress
    label_to_update.set_label(&format!("Translating to {}...", target_language));
    status_label.set_text("Translating...");

    // Call core translation function
    let start_time = std::time::Instant::now();
    match translate_text(
        &text_to_translate,
        target_language,
        api_key,
        api_url,
        model_version,
        &provider_name,
        cancel_receiver,
        logger.as_ref(),
    )
    .await
    {
        Ok(translated_text) => {
            let elapsed = start_time.elapsed();
            if generation == current_generation.get() {
                label_to_update.set_text(&translated_text);
                status_label.set_text(&format!("Translation completed in {:.1}s", elapsed.as_secs_f64()));
            }
        }
        Err(error_message) => {
            // Don't show cancelled messages to avoid confusion
            if error_message != "Translation cancelled" {
                eprintln!("Translation Error: {}", error_message);
                if generation == current_generation.get() {
                    label_to_update.set_text(&error_message);
                }
            }
            let elapsed = start_time.elapsed();
            status_label.set_text(&format!("Translation failed after {:.1}s", elapsed.as_secs_f64()));
        }
    }
}

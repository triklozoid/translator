// Use lingua::Language directly
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use gtk::Label;
use lingua::Language;
use tokio::sync::oneshot;
use crate::logger::Logger;

// Result type for translations
pub type TranslationResult = Result<String, String>;

// Core translation function without UI dependencies with cancellation support
pub async fn translate_text(
    text_to_translate: &str,
    target_language: Language,
    api_key: String,
    api_url: String,
    model_version: String,
    cancel_receiver: Option<oneshot::Receiver<()>>,
    logger: Option<&Logger>,
) -> TranslationResult {
    // Check if text is empty before making API call
    if text_to_translate.trim().is_empty() {
        return Err("Clipboard text is empty.".to_string());
    }

    // Configure API Client using provided URL
    let config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(api_url);

    let client = Client::with_config(config);

    // Prepare the prompt
    let target_language_name = if target_language == Language::Portuguese {
        "European Portuguese"
    } else {
        &target_language.to_string()
    };
    let system_prompt = format!("You are a helpful assistant that translates text into {}. Provide only the translation text and nothing else.", target_language_name);
    let user_prompt = text_to_translate.to_string();

    // Create Translation Request using provided model version
    let request_result = CreateChatCompletionRequestArgs::default()
        .max_tokens(1024u16)
        .model(model_version)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt.clone())
                .build()
                .map_err(|e| format!("Failed to build system message: {}", e))?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_prompt.clone())
                .build()
                .map_err(|e| format!("Failed to build user message: {}", e))?
                .into(),
        ])
        .build();

    match request_result {
        Ok(request) => {
            // Get chat client
            let chat_client = client.chat();
            // Create the API call future
            let api_future = chat_client.create(request);
            
            // Handle cancellation if provided
            match cancel_receiver {
                Some(cancel_rx) => {
                    tokio::select! {
                        result = api_future => {
                            process_api_response(result, logger, &format!("System: {}\nUser: {}", system_prompt, user_prompt), &target_language.to_string())
                        }
                        _ = cancel_rx => {
                            Err("Translation cancelled".to_string())
                        }
                    }
                }
                None => {
                    // No cancellation, proceed normally
                    let result = api_future.await;
                    process_api_response(result, logger, &format!("System: {}\nUser: {}", system_prompt, user_prompt), &target_language.to_string())
                }
            }
        }
        Err(e) => Err(format!("Error building request: {}", e)),
    }
}

// Helper function to process API response
fn process_api_response(
    result: Result<async_openai::types::CreateChatCompletionResponse, OpenAIError>,
    logger: Option<&Logger>,
    prompt: &str,
    target_language: &str,
) -> TranslationResult {
    match result {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                if let Some(translated_text) = &choice.message.content {
                    let response_text = translated_text.trim().to_string();
                    
                    // Log the translation if logger is available
                    if let Some(logger) = logger {
                        logger.log_translation(prompt, &response_text, target_language);
                    }
                    
                    Ok(response_text)
                } else {
                    Err("API returned no translation content.".to_string())
                }
            } else {
                Err("API returned no choices.".to_string())
            }
        }
        Err(e) => {
            // Provide more specific error feedback if possible
            let error_message = match e {
                OpenAIError::ApiError(api_err) => format!(
                    "API Error: {} (Type: {:?}, Code: {:?})",
                    api_err.message, api_err.r#type, api_err.code
                ),
                OpenAIError::Reqwest(req_err) => format!("Network Error: {}", req_err),
                _ => format!("API Error: {}", e),
            };
            Err(error_message)
        }
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
    label_to_update: Label,
    cancel_receiver: Option<oneshot::Receiver<()>>,
    logger: Option<Logger>,
) {
    // Update UI to show translation in progress
    label_to_update.set_label(&format!("Translating to {}...", target_language));

    // Call core translation function
    match translate_text(
        &text_to_translate,
        target_language,
        api_key,
        api_url,
        model_version,
        cancel_receiver,
        logger.as_ref(),
    )
    .await
    {
        Ok(translated_text) => {
            label_to_update.set_text(&translated_text);
        }
        Err(error_message) => {
            // Don't show cancelled messages to avoid confusion
            if error_message != "Translation cancelled" {
                eprintln!("Translation Error: {}", error_message);
                label_to_update.set_text(&error_message);
            }
        }
    }
}

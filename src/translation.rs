// Use lingua::Language directly
use lingua::Language;
use async_openai::{
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client, config::OpenAIConfig, error::OpenAIError,
};
use gtk::Label;

// Result type for translations
pub type TranslationResult = Result<String, String>;

// Core translation function without UI dependencies
pub async fn translate_text(
    text_to_translate: &str,
    target_language: Language,
    api_key: String,
    api_url: String,
    model_version: String,
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

    // Create Translation Request using provided model version
    let request_result = CreateChatCompletionRequestArgs::default()
        .max_tokens(1024u16)
        .model(model_version)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(format!("You are a helpful assistant that translates text into {}. Provide only the translation text and nothing else.", target_language.to_string()))
                .build()
                .map_err(|e| format!("Failed to build system message: {}", e))?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(text_to_translate.to_string())
                .build()
                .map_err(|e| format!("Failed to build user message: {}", e))?
                .into(),
        ])
        .build();

    match request_result {
        Ok(request) => {
            // Call API
            match client.chat().create(request).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.get(0) {
                        if let Some(translated_text) = &choice.message.content {
                            Ok(translated_text.trim().to_string())
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
                        OpenAIError::ApiError(api_err) => format!("API Error: {} (Type: {:?}, Code: {:?})", api_err.message, api_err.r#type, api_err.code),
                        OpenAIError::Reqwest(req_err) => format!("Network Error: {}", req_err),
                        _ => format!("API Error: {}", e),
                    };
                    Err(error_message)
                }
            }
        }
        Err(e) => {
            Err(format!("Error building request: {}", e))
        }
    }
}

// --- Helper function to request translation ---
// UI wrapper around core translation function
pub async fn request_translation(
    text_to_translate: String,
    target_language: Language,
    api_key: String,
    api_url: String,
    model_version: String,
    label_to_update: Label,
) {
    // Update UI to show translation in progress
    label_to_update.set_label(&format!("Translating to {}...", target_language.to_string()));

    // Call core translation function
    match translate_text(
        &text_to_translate,
        target_language,
        api_key,
        api_url,
        model_version
    ).await {
        Ok(translated_text) => {
            label_to_update.set_text(&translated_text);
        }
        Err(error_message) => {
            eprintln!("Translation Error: {}", error_message);
            label_to_update.set_text(&error_message);
        }
    }
}


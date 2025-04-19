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

// --- Helper function to request translation ---
// Added api_url and model_version parameters
// Changed target_language type to lingua::Language
pub async fn request_translation(
    text_to_translate: String,
    target_language: Language, // Use lingua::Language
    api_key: String,
    api_url: String, // Added
    model_version: String, // Added
    label_to_update: Label,
) {
    // Check if text is empty before making API call
    if text_to_translate.trim().is_empty() {
        label_to_update.set_text("Clipboard text is empty.");
        return;
    }

    // Use target_language.to_string() for display name
    label_to_update.set_label(&format!("Translating to {}...", target_language.to_string()));

    // Configure API Client using provided URL
    let config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(api_url); // Use api_url from config

    let client = Client::with_config(config);

    // Create Translation Request using provided model version
    // Use target_language.to_string() in the prompt
    let request_result = CreateChatCompletionRequestArgs::default()
        .max_tokens(1024u16) // Increased token limit slightly
        .model(model_version) // Use model_version from config
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(format!("You are a helpful assistant that translates text into {}. Provide only the translation text and nothing else.", target_language.to_string())) // Use language name string
                .build().expect("Failed to build system message").into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(text_to_translate) // Just pass the text directly
                .build().expect("Failed to build user message").into(),
        ])
        .build();

    match request_result {
        Ok(request) => {
            // Call API
            match client.chat().create(request).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.get(0) {
                        if let Some(translated_text) = &choice.message.content {
                            label_to_update.set_text(translated_text.trim());
                        } else {
                            label_to_update.set_text("API returned no translation content.");
                        }
                    } else {
                        label_to_update.set_text("API returned no choices.");
                    }
                }
                Err(e) => {
                    eprintln!("API Error: {}", e);
                    // Provide more specific error feedback if possible
                    let error_message = match e {
                        OpenAIError::ApiError(api_err) => format!("API Error: {} (Type: {:?}, Code: {:?})", api_err.message, api_err.r#type, api_err.code),
                        OpenAIError::Reqwest(req_err) => format!("Network Error: {}", req_err),
                        _ => format!("API Error: {}", e),
                    };
                    label_to_update.set_text(&error_message);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to build API request: {}", e);
            label_to_update.set_text(&format!("Error building request: {}", e));
        }
    }
}

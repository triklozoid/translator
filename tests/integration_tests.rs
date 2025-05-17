use translator::{
    config::{Config, load_config, save_config},
    settings::{load_last_language, save_last_language},
    translate_text,
    ui::choose_target_language,
};
use lingua::Language;
use std::env;
use tempfile;

#[test]
fn test_config_and_settings_integration() {
    // Create a temporary directory for all configuration
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // 1. Create and save a custom configuration
    let mut config = Config::default();
    config.primary_language = Language::Spanish;
    config.secondary_language = Language::Italian;
    config.api_url = "https://custom.api.example.com".to_string();
    config.all_target_languages.push(Language::Spanish);
    config.all_target_languages.push(Language::Italian);
    
    save_config(&config).expect("Failed to save config");
    
    // 2. Save a last language preference
    save_last_language(Language::Portuguese).expect("Failed to save last language");
    
    // 3. Load the configuration and verify
    let loaded_config = load_config();
    assert_eq!(loaded_config.primary_language, Language::Spanish);
    assert_eq!(loaded_config.secondary_language, Language::Italian);
    assert_eq!(loaded_config.api_url, "https://custom.api.example.com");
    
    // 4. Load the last language and verify
    let loaded_language = load_last_language();
    assert_eq!(loaded_language, Language::Portuguese);
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[tokio::test]
async fn test_translation_workflow() {
    // Test the complete translation workflow
    let test_cases = vec![
        ("", Language::Spanish, true), // Empty text should fail
        ("   ", Language::French, true), // Whitespace should fail
        ("Hello", Language::German, true), // Valid text but invalid API will fail
    ];
    
    for (text, target_language, should_fail) in test_cases {
        let result = translate_text(
            text,
            target_language,
            "test-key".to_string(),
            "http://127.0.0.1:9999".to_string(),
            "test-model".to_string(),
        ).await;
        
        if should_fail {
            assert!(result.is_err(), "Expected failure for text: '{}'", text);
        } else {
            assert!(result.is_ok(), "Expected success for text: '{}'", text);
        }
    }
}

#[test]
fn test_language_selection_with_config() {
    // Create a temporary directory for configuration
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Create custom configuration
    let mut config = Config::default();
    config.primary_language = Language::English;
    config.secondary_language = Language::French;
    save_config(&config).expect("Failed to save config");
    
    // Save a last language preference
    save_last_language(Language::Spanish).expect("Failed to save last language");
    
    // Test language selection algorithm with the saved config
    let test_cases = vec![
        // (source, expected_target)
        (Some(Language::German), Language::English), // Non-primary → primary
        (Some(Language::English), Language::Spanish), // Primary → last choice
        (None, Language::English), // No source → primary
    ];
    
    for (source, expected) in test_cases {
        let target = choose_target_language(
            source,
            config.primary_language,
            config.secondary_language,
            load_last_language()
        );
        assert_eq!(target, expected, "Failed for source: {:?}", source);
    }
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[test]
fn test_full_application_flow() {
    // This test simulates a complete application flow
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // 1. Load default configuration (first run)
    let initial_config = load_config();
    assert_eq!(initial_config.primary_language, Language::English);
    assert_eq!(initial_config.secondary_language, Language::French);
    
    // 2. User changes primary language
    let mut updated_config = initial_config.clone();
    updated_config.primary_language = Language::German;
    save_config(&updated_config).expect("Failed to save updated config");
    
    // 3. User selects a target language
    save_last_language(Language::Italian).expect("Failed to save last language");
    
    // 4. Simulate language selection for translation
    let source_text_lang = Some(Language::French);
    let target_lang = choose_target_language(
        source_text_lang,
        updated_config.primary_language,
        updated_config.secondary_language,
        load_last_language()
    );
    
    // Since source is not primary (German), should choose primary
    assert_eq!(target_lang, Language::German);
    
    // 5. Verify configuration persists
    let reloaded_config = load_config();
    assert_eq!(reloaded_config.primary_language, Language::German);
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[tokio::test]
async fn test_error_handling_integration() {
    // Test error handling across different components
    
    // 1. Translation with invalid API configuration
    let result = translate_text(
        "Test text",
        Language::Spanish,
        "".to_string(), // Empty API key
        "http://invalid.url".to_string(),
        "invalid-model".to_string(),
    ).await;
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Error") || error.contains("Network"));
    
    // 2. Config with invalid path (handled gracefully)
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", "/nonexistent/path");
    
    // Should return default config when path is invalid
    let config = load_config();
    assert_eq!(config.primary_language, Language::English);
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}
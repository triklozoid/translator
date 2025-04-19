use lingua::{Language, IsoCode639_1};
use std::str::FromStr;
use std::path::PathBuf;
use std::fs;
use std::env;

// Import the crate to test
use translator::settings::{load_last_language, save_last_language};

#[test]
fn test_save_load_last_language() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Test languages
    let test_languages = vec![
        Language::German,
        Language::Spanish,
        Language::Italian,
        Language::French
    ];
    
    for lang in test_languages {
        // Save the language
        save_last_language(lang).expect("Failed to save language");
        
        // Load the language
        let loaded_lang = load_last_language();
        
        // Check that loaded language matches saved language
        assert_eq!(loaded_lang, lang, "Loaded language should match saved language");
    }
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[test]
fn test_load_last_language_default() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    
    // Set the config directory for this test to ensure no existing config is used
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Load language without saving first (should return default)
    let default_lang = load_last_language();
    
    // Default should be English
    assert_eq!(default_lang, Language::English, "Default language should be English");
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

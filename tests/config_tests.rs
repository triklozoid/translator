use lingua::{Language, IsoCode639_1};
use std::str::FromStr;
use std::path::PathBuf;
use std::fs;
use std::env;
use std::io::Write;

// Import the crate to test
use translator::config::{Config, load_config, save_config};

#[test]
fn test_config_default() {
    // Test that default config has expected values
    let config = Config::default();
    
    // Check default languages
    assert_eq!(config.primary_language, Language::from_iso_code_639_1(&IsoCode639_1::from_str("EN").unwrap()));
    assert_eq!(config.secondary_language, Language::from_iso_code_639_1(&IsoCode639_1::from_str("FR").unwrap()));
    
    // Check API settings
    assert_eq!(config.api_url, "https://openrouter.ai/api/v1");
    assert_eq!(config.model_version, "openai/gpt-4o");
    
    // Check target languages
    assert!(config.all_target_languages.contains(&Language::English));
    assert!(config.all_target_languages.contains(&Language::French));
    assert!(config.all_target_languages.contains(&Language::Italian));
    assert!(config.all_target_languages.contains(&Language::Polish));
}

#[test]
fn test_config_serialization() {
    // Create a test config
    let mut config = Config::default();
    config.primary_language = Language::German;
    config.secondary_language = Language::Spanish;
    
    // Serialize to TOML
    let toml_string = toml::to_string_pretty(&config).expect("Failed to serialize config");
    
    // Check that serialization worked correctly
    assert!(toml_string.contains("primary_language"));
    assert!(toml_string.contains("DE")); // German ISO code
    assert!(toml_string.contains("ES")); // Spanish ISO code
    
    // Deserialize back
    let deserialized: Config = toml::from_str(&toml_string).expect("Failed to deserialize config");
    
    // Check that values match
    assert_eq!(deserialized.primary_language, Language::German);
    assert_eq!(deserialized.secondary_language, Language::Spanish);
}

#[test]
fn test_config_save_load() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let config_dir = temp_dir.path().join("translator");
    fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Create a custom config
    let mut config = Config::default();
    config.primary_language = Language::Italian;
    config.secondary_language = Language::Polish;
    config.api_url = "https://test-api.example.com".to_string();
    
    // Save the config
    save_config(&config).expect("Failed to save config");
    
    // Load the config
    let loaded_config = load_config();
    
    // Check that loaded config matches saved config
    assert_eq!(loaded_config.primary_language, Language::Italian);
    assert_eq!(loaded_config.secondary_language, Language::Polish);
    assert_eq!(loaded_config.api_url, "https://test-api.example.com");
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

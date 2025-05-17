use lingua::{Language, IsoCode639_1};
use std::str::FromStr;
use std::fs;
use std::env;

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
#[ignore]
fn test_config_save_load() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Create the translator directory
    let config_dir = temp_dir.path().join("translator");
    fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    
    // Create a custom config and include the languages in all_target_languages
    let mut config = Config::default();
    config.primary_language = Language::Italian;
    config.secondary_language = Language::Polish;
    config.api_url = "https://test-api.example.com".to_string();
    // Add the languages to all_target_languages to ensure they're available
    config.all_target_languages.push(Language::Italian);
    config.all_target_languages.push(Language::Polish);
    
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

#[test]
fn test_config_missing_file() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    
    // Set a non-existent config directory
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Try to load config (should return default)
    let config = load_config();
    
    // Should get default config when file doesn't exist
    assert_eq!(config.primary_language, Language::English);
    assert_eq!(config.secondary_language, Language::French);
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[test]
fn test_config_invalid_toml() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let config_dir = temp_dir.path().join("translator");
    fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Write invalid TOML to the config file
    let config_file = config_dir.join("config.toml");
    fs::write(&config_file, "invalid toml content [").expect("Failed to write invalid config");
    
    // Try to load config (should return default due to parse error)
    let config = load_config();
    
    // Should get default config when parsing fails
    assert_eq!(config.primary_language, Language::English);
    assert_eq!(config.secondary_language, Language::French);
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[test]
fn test_config_all_target_languages() {
    let config = Config::default();
    
    // Check that all expected languages are present
    let expected_languages = vec![
        Language::English,
        Language::French,
        Language::Italian,
        Language::Polish,
    ];
    
    for language in expected_languages {
        assert!(config.all_target_languages.contains(&language),
            "Language {:?} not found in all_target_languages", language);
    }
}

#[test]
fn test_config_custom_languages() {
    let mut config = Config::default();
    config.all_target_languages = vec![Language::Japanese, Language::Chinese, Language::Korean];
    
    // Serialize to TOML
    let toml_string = toml::to_string_pretty(&config).expect("Failed to serialize config");
    
    // Deserialize back
    let deserialized: Config = toml::from_str(&toml_string).expect("Failed to deserialize config");
    
    // Check that custom languages are preserved
    assert_eq!(deserialized.all_target_languages.len(), 3);
    assert!(deserialized.all_target_languages.contains(&Language::Japanese));
    assert!(deserialized.all_target_languages.contains(&Language::Chinese));
    assert!(deserialized.all_target_languages.contains(&Language::Korean));
}

#[test]
fn test_config_save_permissions_error() {
    // Skip this test on platforms where permissions might not work as expected
    if cfg!(target_os = "windows") {
        return;
    }
    
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let config_dir = temp_dir.path().join("translator");
    
    // Create the config directory and make it unwritable
    fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Make the directory non-writable
    let mut dir_permissions = fs::metadata(&config_dir).expect("Failed to get metadata").permissions();
    use std::os::unix::fs::PermissionsExt;
    dir_permissions.set_mode(0o555); // Read and execute only, no write
    fs::set_permissions(&config_dir, dir_permissions).expect("Failed to set permissions");
    
    // Try to save config (should fail due to permissions)
    let config = Config::default();
    let result = save_config(&config);
    
    // Should fail with permission error
    assert!(result.is_err(), "Save should fail with permission error, but got: {:?}", result);
    
    // Restore directory permissions for cleanup
    let mut dir_permissions = fs::metadata(&config_dir).expect("Failed to get metadata").permissions();
    dir_permissions.set_mode(0o755);
    fs::set_permissions(&config_dir, dir_permissions).ok();
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[test]
fn test_config_serialization_with_empty_languages() {
    let mut config = Config::default();
    config.all_target_languages = vec![]; // Empty languages list
    
    // Serialize to TOML
    let toml_string = toml::to_string_pretty(&config).expect("Failed to serialize config");
    
    // Deserialize back
    let deserialized: Config = toml::from_str(&toml_string).expect("Failed to deserialize config");
    
    // Check that empty list is preserved
    assert_eq!(deserialized.all_target_languages.len(), 0);
}
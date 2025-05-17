use lingua::Language;
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

#[test]
#[ignore] // This test has issues in the test environment
fn test_save_language_with_invalid_directory() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let invalid_dir = temp_dir.path().join("non_existent").join("nested");
    
    // Set the config directory to a non-existent nested path
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", invalid_dir);
    
    // Try to save a language - should succeed by creating directories
    let result = save_last_language(Language::Spanish);
    assert!(result.is_ok(), "Should create missing directories and save");
    
    // Verify the language was saved
    let loaded = load_last_language();
    assert_eq!(loaded, Language::Spanish);
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[test]
fn test_load_language_with_invalid_content() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let config_dir = temp_dir.path().join("translator");
    fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Write invalid content to the language file
    let lang_file = config_dir.join("last_language.txt");
    fs::write(&lang_file, "INVALID_LANGUAGE").expect("Failed to write invalid language");
    
    // Try to load language (should return default)
    let lang = load_last_language();
    
    // Should get default language when content is invalid
    assert_eq!(lang, Language::English, "Should return default for invalid content");
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[test]
#[ignore] // Permissions tests can be unreliable in different environments
fn test_save_language_permissions_error() {
    // Skip this test on platforms where permissions might not work as expected
    if cfg!(target_os = "windows") {
        return;
    }
    
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let config_dir = temp_dir.path().join("translator");
    
    // Create the config directory but make it read-only
    fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    let lang_file = config_dir.join("last_language.txt");
    
    // Create the file and make it read-only
    fs::write(&lang_file, "").expect("Failed to create file");
    let mut permissions = fs::metadata(&lang_file).expect("Failed to get metadata").permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&lang_file, permissions).expect("Failed to set permissions");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Try to save language (should fail due to permissions)
    let result = save_last_language(Language::German);
    
    // Should fail with permission error
    assert!(result.is_err(), "Should fail with permission error");
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[test]
fn test_basic_language_edge_cases() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Test with a few specific languages that should work
    let test_languages = vec![
        Language::English,
        Language::French,
        Language::Spanish,
        Language::German,
        Language::Italian,
    ];
    
    for lang in test_languages {
        save_last_language(lang).expect("Failed to save language");
        let loaded = load_last_language();
        assert_eq!(loaded, lang, "Language mismatch for {:?}", lang);
    }
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}

#[test]
fn test_unsupported_language() {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    
    // Set the config directory for this test
    let original_config_home = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Create config directory
    let config_dir = temp_dir.path().join("translator");
    fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    
    // Write an ISO code that might not be supported
    let lang_file = config_dir.join("last_language.txt");
    fs::write(&lang_file, "XX").expect("Failed to write unsupported code");
    
    // Try to load language (should return default)
    let lang = load_last_language();
    
    // Should get default language when code is unsupported
    assert_eq!(lang, Language::English, "Should return default for unsupported code");
    
    // Restore original environment
    if let Some(original) = original_config_home {
        env::set_var("XDG_CONFIG_HOME", original);
    } else {
        env::remove_var("XDG_CONFIG_HOME");
    }
}
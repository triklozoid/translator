use crate::language::TargetLanguage;
use std::env;
use std::fs;
use std::path::PathBuf;

const SETTINGS_FILE: &str = "translator_last_lang.txt"; // Имя файла для сохранения

// --- Helper function to get settings file path ---
fn get_settings_path() -> PathBuf {
    let mut path = env::temp_dir();
    path.push(SETTINGS_FILE);
    path
}

// --- Helper function to load language from settings ---
pub fn load_language_setting() -> Option<TargetLanguage> {
    let path = get_settings_path();
    match fs::read_to_string(path) {
        Ok(code) => TargetLanguage::from_code(code.trim()),
        Err(e) => {
            // Don't print error if file simply doesn't exist, that's expected on first run
            if e.kind() != std::io::ErrorKind::NotFound {
                println!("Could not load language setting: {}", e); // Log other errors
            }
            None
        }
    }
}

// --- Helper function to save language to settings ---
pub fn save_language_setting(lang: TargetLanguage) {
    let path = get_settings_path();
    if let Err(e) = fs::write(path, lang.code()) {
        eprintln!("Failed to save language setting: {}", e); // Use eprintln for errors
    }
}

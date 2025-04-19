// Use lingua::Language and IsoCode639_1 directly
use lingua::{Language, IsoCode639_1};
use std::str::FromStr;
use std::fs;
use std::path::PathBuf;

const SETTINGS_DIR: &str = "translator";
const LAST_LANG_FILE: &str = "last_language.txt"; // Store ISO code

// --- Helper function to get last language file path ---
fn get_last_lang_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut path| {
        path.push(SETTINGS_DIR);
        path.push(LAST_LANG_FILE);
        path
    })
}

// --- Helper function to load last language from settings ---
// Returns lingua::Language
pub fn load_last_language() -> Language {
    // Default to English if no saved language
    let default_language = Language::from_iso_code_639_1(&IsoCode639_1::from_str("EN").unwrap());
    match get_last_lang_path() {
        Some(path) => {
            match fs::read_to_string(path) {
                Ok(iso_code) => {
                    // Try to convert the ISO code to a Language
                    let iso_code_str = iso_code.trim().to_uppercase();
                    println!("Loaded last language ISO code: {}", iso_code_str);
                    
                    // Try to parse the string as an IsoCode639_1 enum value
                    match IsoCode639_1::from_str(&iso_code_str) {
                        Ok(iso_code) => {
                            // Convert from IsoCode639_1 to Language
                            let lang = Language::from_iso_code_639_1(&iso_code);
                            println!("Loaded last language: {:?}", lang);
                            lang
                        },
                        Err(_) => {
                            // Try to parse as language name for backward compatibility
                            match Language::from_str(&iso_code_str) {
                                Ok(lang) => lang,
                                Err(_) => {
                                    println!("Invalid ISO code '{}' in settings file, using default {:?}", iso_code_str, default_language);
                                    default_language
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    // Don't print error if file simply doesn't exist, that's expected on first run
                    if e.kind() != std::io::ErrorKind::NotFound {
                        println!("Could not load language setting: {}", e); // Log other errors
                    }
                    default_language // Default if file can't be read
                }
            }
        },
        None => {
            println!("Could not determine config directory for last language");
            default_language // Default if path can't be determined
        }
    }
}

// --- Helper function to save last language to settings ---
// Accepts lingua::Language
pub fn save_last_language(lang: Language) -> Result<(), std::io::Error> {
    let path = get_last_lang_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine config directory for last language",
        )
    })?;

    // Create the parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?; // Propagate IO errors
    }

    // Get the ISO 639-1 code for the language
    let iso_code = lang.iso_code_639_1().to_string().to_uppercase();

    // Use temp file writing to avoid corrupting the file if saving is interrupted
    let temp_path = path.with_extension("tmp");
    // Write the ISO code (e.g., "EN")
    fs::write(&temp_path, &iso_code)?;

    // Rename the temporary file to the final file name
    fs::rename(&temp_path, &path)?;

    println!("Last language saved to {:?}: {:?} (ISO: {})", path, lang, iso_code);
    Ok(())
}

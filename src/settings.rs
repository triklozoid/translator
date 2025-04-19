// Use lingua::Language directly
use lingua::Language;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr; // Required for Language::from_str

const SETTINGS_DIR: &str = "translator";
const LAST_LANG_FILE: &str = "last_language.txt"; // Store language name string

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
    let default_language = Language::English; // Default language
    match get_last_lang_path() {
        Some(path) => {
            match fs::read_to_string(path) {
                Ok(lang_name) => {
                    // Try to parse the string into a Language enum variant
                    println!("Loaded last language string: {}", lang_name);
                    match lang_name.trim().parse::<Language>() {
                        Ok(lang) => {
                            println!("Loaded last language: {:?}", lang);
                            lang
                        },
                        Err(_) => {
                            // Fallback to mapping two-letter language codes
                            match lang_name.trim().to_uppercase().as_str() {
                                "UK" => {
                                    println!("Mapped two-letter code 'UK' to Ukrainian.");
                                    Language::Ukrainian
                                },
                                "EN" => {
                                    println!("Mapped two-letter code 'EN' to English.");
                                    Language::English
                                },
                                "RU" => {
                                    println!("Mapped two-letter code 'RU' to Russian.");
                                    Language::Russian
                                },
                                "PT" => {
                                    println!("Mapped two-letter code 'PT' to Portuguese.");
                                    Language::Portuguese
                                },
                                _ => {
                                    println!("Invalid language name '{}' in settings file, using default {:?}", lang_name.trim(), default_language);
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

    // Use temp file writing to avoid corrupting the file if saving is interrupted
    let temp_path = path.with_extension("tmp");
    // Write the language name string (e.g., "English")
    fs::write(&temp_path, lang.to_string())?;

    // Rename the temporary file to the final file name
    fs::rename(&temp_path, &path)?;

    println!("Last language saved to {:?}: {:?}", path, lang);
    Ok(())
}

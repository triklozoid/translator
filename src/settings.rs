use crate::language::TargetLanguage;
use std::fs;
use std::path::PathBuf;

const SETTINGS_DIR: &str = "translator";
const LAST_LANG_FILE: &str = "last_language.txt";

// --- Helper function to get last language file path ---
fn get_last_lang_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut path| {
        path.push(SETTINGS_DIR);
        path.push(LAST_LANG_FILE);
        path
    })
}

// --- Helper function to load last language from settings ---
pub fn load_last_language() -> TargetLanguage {
    match get_last_lang_path() {
        Some(path) => {
            match fs::read_to_string(path) {
                Ok(code) => {
                    match TargetLanguage::from_code(code.trim()) {
                        Some(lang) => {
                            println!("Loaded last language: {:?}", lang);
                            lang
                        },
                        None => {
                            println!("Invalid language code in settings file, using default");
                            TargetLanguage::English // Default if code is invalid
                        }
                    }
                },
                Err(e) => {
                    // Don't print error if file simply doesn't exist, that's expected on first run
                    if e.kind() != std::io::ErrorKind::NotFound {
                        println!("Could not load language setting: {}", e); // Log other errors
                    }
                    TargetLanguage::English // Default if file can't be read
                }
            }
        },
        None => {
            println!("Could not determine config directory for last language");
            TargetLanguage::English // Default if path can't be determined
        }
    }
}

// --- Helper function to save last language to settings ---
pub fn save_last_language(lang: TargetLanguage) -> Result<(), std::io::Error> {
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
    fs::write(&temp_path, lang.code())?;
    
    // Rename the temporary file to the final file name
    fs::rename(&temp_path, &path)?;

    println!("Last language saved to {:?}: {:?}", path, lang);
    Ok(())
}

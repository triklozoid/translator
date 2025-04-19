// Use lingua::Language directly
use lingua::{Language, IsoCode639_1};
use std::str::FromStr;
use serde::{Deserialize, Serialize, Deserializer, Serializer}; // Import necessary serde traits
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH}; // For timestamp in backup filename

const CONFIG_DIR: &str = "translator";
const CONFIG_FILE: &str = "config.toml";

// --- Serde helper module for lingua::Language ---
mod language_serde {
    use super::*; // Import items from parent module (Language, etc.)
    use serde::de::Error; // Import serde error type

    // Serialize a single Language to its ISO code
    pub fn serialize<S>(lang: &Language, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Get the ISO 639-1 code and convert to uppercase string
        let code = lang.iso_code_639_1().to_string().to_uppercase();
        serializer.serialize_str(&code)
    }

    // Deserialize a single Language from its ISO code
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Language, D::Error>
    where
        D: Deserializer<'de>,
    {
        use lingua::IsoCode639_1;
        let code = String::deserialize(deserializer)?;
        
        // Try to parse as ISO code first
        if let Ok(iso_code) = IsoCode639_1::from_str(&code.to_uppercase()) {
            // Convert from IsoCode639_1 to Language
            return Ok(Language::from_iso_code_639_1(&iso_code));
        }
        
        // If that fails, try to parse as language name (for backward compatibility)
        Language::from_str(&code)
            .map_err(|_| D::Error::custom(format!("invalid language code or name: {}", code)))
    }

    // --- Helpers for Vec<Language> ---
    // We need separate helpers for Vec because #[serde(with = "...")] applies to the whole field

    // Serialize Vec<Language> to ISO codes
    pub fn serialize_vec<S>(langs: &[Language], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(langs.len()))?;
        for lang in langs {
            // Use ISO code for each language
            let code = lang.iso_code_639_1().to_string().to_uppercase();
            seq.serialize_element(&code)?;
        }
        seq.end()
    }

    // Deserialize Vec<Language> from ISO codes
    pub fn deserialize_vec<'de, D>(deserializer: D) -> Result<Vec<Language>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use lingua::IsoCode639_1;
        let codes: Vec<String> = Vec::deserialize(deserializer)?;
        codes
            .into_iter()
            .map(|code| {
                // Try to parse as ISO code first
                if let Ok(iso_code) = IsoCode639_1::from_str(&code.to_uppercase()) {
                    return Ok(Language::from_iso_code_639_1(&iso_code));
                }
                
                // If that fails, try to parse as language name (for backward compatibility)
                Language::from_str(&code)
                    .map_err(|_| D::Error::custom(format!("invalid language code or name in list: {}", code)))
            })
            .collect() // Collect results into Result<Vec<Language>, D::Error>
    }
}


// Derive Serialize, Deserialize, Debug, and Clone for the Config struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub model_version: String,
    // Use lingua::Language with serde helpers
    #[serde(with = "language_serde")] // Use the helper module for single Language
    pub primary_language: Language,
    #[serde(with = "language_serde")] // Use the helper module for single Language
    pub secondary_language: Language,
    // List of available target languages for the UI
    #[serde(default = "default_all_target_languages")] // Use default if missing in file
    #[serde(serialize_with = "language_serde::serialize_vec")] // Use specific vec serializer
    #[serde(deserialize_with = "language_serde::deserialize_vec")] // Use specific vec deserializer
    pub all_target_languages: Vec<Language>,
}

// Function to provide default value for all_target_languages
// Needs to be a separate function for use with #[serde(default = "...")]
// Provide a sensible subset of languages, not all 75+
fn default_all_target_languages() -> Vec<Language> {
    vec![
        Language::from_iso_code_639_1(&IsoCode639_1::from_str("EN").unwrap()), // English
        Language::from_iso_code_639_1(&IsoCode639_1::from_str("FR").unwrap()), // French
        Language::from_iso_code_639_1(&IsoCode639_1::from_str("IT").unwrap()), // Italian
        Language::from_iso_code_639_1(&IsoCode639_1::from_str("PL").unwrap()), // Polish
    ]
}


impl Default for Config {
    fn default() -> Self {
        // Create default languages using ISO codes for consistency
        let primary = Language::from_iso_code_639_1(&IsoCode639_1::from_str("EN").unwrap());
        let secondary = Language::from_iso_code_639_1(&IsoCode639_1::from_str("FR").unwrap());
        
        Config {
            api_url: "https://openrouter.ai/api/v1".to_string(),
            model_version: "openai/gpt-4o".to_string(),
            primary_language: primary,
            secondary_language: secondary,
            all_target_languages: default_all_target_languages(),
        }
    }
}


// --- Configuration Loading and Saving ---

fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut path| {
        path.push(CONFIG_DIR);
        path.push(CONFIG_FILE);
        path
    })
}

pub fn load_config() -> Config {
    match get_config_path() {
        Some(path) => {
            if !path.exists() {
                println!(
                    "Config file not found at {:?}. Creating with defaults.",
                    path
                );
                let default_config = Config::default();
                // Attempt to save the default config immediately
                if let Err(e) = save_config(&default_config) {
                    eprintln!("Failed to save default config: {}", e);
                    // Continue with default config even if saving failed initially
                }
                return default_config;
            }

            match fs::File::open(&path) {
                Ok(mut file) => {
                    let mut contents = String::new();
                    if let Err(e) = file.read_to_string(&mut contents) {
                        eprintln!("Failed to read config file {:?}: {}. Using defaults.", path, e);
                        return Config::default(); // Return default on read error
                    }

                    // Attempt to parse.
                    match toml::from_str::<Config>(&contents) {
                        Ok(mut config) => {
                            println!("Successfully loaded config from {:?}", path); // Log success
                            // Ensure all_target_languages is not empty, use default if it is
                            // (Should be handled by serde(default), but as a fallback)
                            if config.all_target_languages.is_empty() {
                                println!("Warning: 'all_target_languages' was empty in config file, using default list.");
                                config.all_target_languages = default_all_target_languages();
                            }
                            // Ensure primary/secondary languages are actually in the list
                            // (Optional validation, could also just let it be)
                            if !config.all_target_languages.contains(&config.primary_language) {
                                eprintln!("Warning: Primary language '{:?}' from config is not in 'all_target_languages'.", config.primary_language);
                                // Optionally add it or reset to default? For now, just warn.
                            }
                             if !config.all_target_languages.contains(&config.secondary_language) {
                                eprintln!("Warning: Secondary language '{:?}' from config is not in 'all_target_languages'.", config.secondary_language);
                            }

                            // Log the loaded languages for debugging
                            println!("Loaded 'primary_language': {:?}", config.primary_language);
                            println!("Loaded 'secondary_language': {:?}", config.secondary_language);
                            println!("Loaded 'all_target_languages': {:?}", config.all_target_languages.iter().map(|l| l.to_string()).collect::<Vec<_>>());
                            config
                        },
                        Err(e) => {
                            // Print the detailed parsing error
                            eprintln!("Failed to parse config file {:?}. Using defaults.", path);
                            eprintln!("Parsing Error: {}", e);

                            // --- Backup invalid config file ---
                            let backup_path = path.with_extension({
                                let timestamp = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0);
                                format!("toml.invalid_{}", timestamp)
                            });
                            eprintln!("Backing up invalid config to {:?}", backup_path);
                            if let Err(backup_err) = fs::rename(&path, &backup_path) {
                                eprintln!("Failed to backup invalid config file: {}", backup_err);
                            }
                            // --- End backup ---

                            // Create and save a default config file after backing up the invalid one
                            println!("Creating a new default config file at {:?}", path);
                            let default_config = Config::default();
                            if let Err(save_err) = save_config(&default_config) {
                                eprintln!("Failed to save new default config: {}", save_err);
                            }
                            default_config // Return default config
                        }
                    }
                }
                Err(e) => {
                    // Handle specific errors like permission denied differently if needed
                    eprintln!("Failed to open config file {:?}: {}. Using defaults.", path, e);
                    Config::default() // Return default on open error
                }
            }
        }
        None => {
            eprintln!("Could not determine config directory. Using defaults.");
            Config::default() // Return default if config dir is unknown
        }
    }
}

pub fn save_config(config: &Config) -> Result<(), std::io::Error> {
    let path = get_config_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine config directory",
        )
    })?;

    // Create the parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?; // Propagate IO errors
    }

    // Validate before saving (optional, but good practice)
    let mut validated_config = config.clone();
    if validated_config.all_target_languages.is_empty() {
        println!("Warning: 'all_target_languages' is empty during save, restoring defaults.");
        validated_config.all_target_languages = default_all_target_languages();
    }
    // Ensure primary/secondary are in the list (optional: add them if missing?)
    if !validated_config.all_target_languages.contains(&validated_config.primary_language) {
         eprintln!("Warning: Primary language {:?} not in list during save. Adding it.", validated_config.primary_language);
         validated_config.all_target_languages.push(validated_config.primary_language);
    }
     if !validated_config.all_target_languages.contains(&validated_config.secondary_language) {
         eprintln!("Warning: Secondary language {:?} not in list during save. Adding it.", validated_config.secondary_language);
         validated_config.all_target_languages.push(validated_config.secondary_language);
    }


    let toml_string = toml::to_string_pretty(&validated_config) // Save the validated config
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("TOML serialization error: {}", e)))?;

    // Use temp file writing to avoid corrupting the file if saving is interrupted
    let temp_path = path.with_extension("tmp");
    { // Scope for file writing
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(toml_string.as_bytes())?;
        file.sync_all()?; // Ensure data is written to disk
    } // File is closed here

    // Rename the temporary file to the final config file name
    fs::rename(&temp_path, &path)?;

    println!("Config saved to {:?}", path); // Log success
    Ok(())
}

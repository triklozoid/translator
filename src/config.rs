use crate::language::TargetLanguage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH}; // For timestamp in backup filename

const CONFIG_DIR: &str = "translator";
const CONFIG_FILE: &str = "config.toml";

// Derive Serialize, Deserialize, Debug, and Clone for the Config struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub model_version: String,
    pub last_target_language: TargetLanguage,
    pub primary_language: TargetLanguage,
    pub secondary_language: TargetLanguage,
    // Added list of all available target languages for the UI
    #[serde(default = "default_all_target_languages")] // Use default if missing in file
    pub all_target_languages: Vec<TargetLanguage>,
}

// Function to provide default value for all_target_languages
// Needs to be a separate function for use with #[serde(default = "...")]
fn default_all_target_languages() -> Vec<TargetLanguage> {
    vec![
        TargetLanguage::Portuguese,
        TargetLanguage::English,
        TargetLanguage::Ukrainian,
        TargetLanguage::Russian,
    ]
}


impl Default for Config {
    fn default() -> Self {
        Config {
            api_url: "https://openrouter.ai/api/v1".to_string(),
            model_version: "openai/gpt-4o-2024-11-20".to_string(),
            last_target_language: TargetLanguage::English,
            primary_language: TargetLanguage::Russian,
            secondary_language: TargetLanguage::English,
            // Use the default function here as well
            all_target_languages: default_all_target_languages(),
        }
    }
}

// --- Implement Serialize/Deserialize for TargetLanguage ---
// We'll serialize/deserialize using the language code (e.g., "EN", "RU")

impl Serialize for TargetLanguage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for TargetLanguage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TargetLanguage::from_code(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid language code: {}", s)))
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

                    // Attempt to parse. If it fails, it might be an old format or missing fields.
                    match toml::from_str::<Config>(&contents) {
                        Ok(mut config) => {
                            println!("Successfully loaded config from {:?}", path); // Log success
                            // Ensure all_target_languages is not empty, use default if it is
                            // (Should be handled by serde(default), but as a fallback)
                            if config.all_target_languages.is_empty() {
                                println!("Warning: 'all_target_languages' was empty in config file, using default list.");
                                config.all_target_languages = default_all_target_languages();
                            }
                            // Ensure last_target_language is within all_target_languages
                            // If not, reset it to the first language in the list or primary/secondary?
                            if !config.all_target_languages.contains(&config.last_target_language) {
                                let new_last_target = config.all_target_languages.first().cloned().unwrap_or_else(|| {
                                    // Fallback if all_target_languages is somehow still empty
                                    eprintln!("Error: 'all_target_languages' is empty even after default checks.");
                                    TargetLanguage::English // Absolute fallback
                                });
                                println!(
                                    "Warning: 'last_target_language' ({:?}) not found in 'all_target_languages'. Resetting to {:?}.",
                                    config.last_target_language, new_last_target
                                );
                                config.last_target_language = new_last_target;
                                // Optionally re-save config
                            }
                            // Log the loaded languages for debugging
                            println!("Loaded 'all_target_languages': {:?}", config.all_target_languages);
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

                            Config::default() // Return default on parse error
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

    let toml_string = toml::to_string_pretty(config)
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

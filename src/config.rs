use crate::language::TargetLanguage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

const CONFIG_DIR: &str = "translator";
const CONFIG_FILE: &str = "config.toml";

// Derive Serialize, Deserialize, Debug, and Clone for the Config struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub model_version: String,
    // Use TargetLanguage directly, implement Serialize/Deserialize for it
    pub language: TargetLanguage,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            // Note: Adjust the default API URL if async-openai uses a different one by default
            // Or read from environment variables as a fallback?
            api_url: "https://api.openrouter.ai/api/v1".to_string(), // Example OpenRouter URL
            // Note: Adjust the default model if needed
            model_version: "openai/gpt-3.5-turbo".to_string(), // Example OpenRouter model identifier
            language: TargetLanguage::English, // Default language
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

                    match toml::from_str(&contents) {
                        Ok(config) => config,
                        Err(e) => {
                            eprintln!("Failed to parse config file {:?}: {}. Using defaults.", path, e);
                            // Consider backing up the invalid config file here
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

    let mut file = fs::File::create(&path)?; // Create or truncate the file
    file.write_all(toml_string.as_bytes())?;
    println!("Config saved to {:?}", path); // Log success
    Ok(())
}

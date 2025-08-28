use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;

#[derive(Clone)]
pub struct Logger {
    enabled: bool,
    log_path: PathBuf,
}

impl Logger {
    pub fn new(enabled: bool) -> Self {
        let log_path = Self::get_log_path();
        
        // Create log directory if it doesn't exist
        if enabled {
            if let Some(parent) = log_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
        }
        
        Logger { enabled, log_path }
    }
    
    fn get_log_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        path.push("translator");
        path.push("app.log");
        path
    }
    
    pub fn log_translation(&self, prompt: &str, response: &str, target_language: &str) {
        if !self.enabled {
            return;
        }
        
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let log_entry = format!(
            "\n[{}] Translation to {}\nPrompt: {}\nResponse: {}\n{}\n",
            timestamp,
            target_language,
            prompt,
            response,
            "-".repeat(80)
        );
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            let _ = writeln!(file, "{}", log_entry);
        }
    }
}
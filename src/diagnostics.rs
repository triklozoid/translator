//! Diagnostic logging — writes to both stderr and app.log unconditionally.
//! Unlike `Logger`, this is always active regardless of the `debug` flag.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Direct function call for the diagnostic log — always active.
pub fn log_diag(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] {}", timestamp, msg);
    // Write to stderr (visible when run from terminal)
    eprintln!("{}", line);
    // Write to app.log (persists regardless of launch method)
    let mut log_path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
    log_path.push("translator");
    log_path.push("app.log");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "{}", line);
        let _ = file.flush();
    }
}

/// Log a diagnostic message — always active, writes to stderr + app.log.
#[macro_export]
macro_rules! diag_log {
    ($($arg:tt)*) => {
        $crate::diagnostics::log_diag(&format!($($arg)*))
    };
}

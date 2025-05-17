// Declare modules
mod config;
mod settings;
mod translation;
mod ui;

use dotenvy::dotenv;
use gtk::prelude::*;
use gtk::{glib, Application};

const APP_ID: &str = "org.gtk_rs.ClipboardTranslator";

// Use tokio runtime for async operations
#[tokio::main]
async fn main() -> glib::ExitCode {
    // Load environment variables from .env file if present
    dotenv().ok(); // This is still useful for API keys, etc.

    // Load configuration from file (or defaults if not found/invalid)
    let config = config::load_config();

    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Clone the config to move into the closure
    let initial_config = config.clone();

    // Connect to "activate" signal of `app`
    // Pass the loaded initial config to the UI builder using a closure
    app.connect_activate(move |app| {
        ui::build_ui(app, initial_config.clone()); // Pass the config
    });

    // Run the application
    app.run()
}

// Helper macro for cloning Rc variables for closures
// Keep it here or move to a dedicated utils module if needed elsewhere
#[macro_export] // Export macro to make it visible in other modules like ui.rs
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident $(: $t:ty)?),+ => $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            $body
        }
    );
    // This variant clones ONE variable using @strong syntax
    (@strong $n:ident => $body:expr) => (
        {
            let $n = $n.clone();
            $body
        }
    );
     (@weak $n:ident => $body:expr) => (
        {
            let $n = $n.downgrade();
            $body
        }
    );
     (@weak $n:ident $(: $t:ty)? = $e:expr => $body:expr) => (
        {
            let $n = $e.downgrade();
            $body
        }
    );
}
// No need for `use clone;` here as the macro is defined in the same scope (root of the crate)
// Other modules will need `use crate::clone;`

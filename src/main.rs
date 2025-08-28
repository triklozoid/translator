// Declare modules
mod config;
mod logger;
mod settings;
mod translation;
mod ui;

use clap::Parser;
use dotenvy::dotenv;
use gtk::prelude::*;
use gtk::{glib, Application};
use single_instance::SingleInstance;

const APP_ID: &str = "org.gtk_rs.ClipboardTranslator";

#[derive(Parser, Debug)]
#[command(name = "translator")]
#[command(about = "A clipboard translator application", long_about = None)]
struct Args {
    /// Enable debug mode to log prompts and LLM responses
    #[arg(long)]
    debug: bool,
}

// Use tokio runtime for async operations
#[tokio::main]
async fn main() -> glib::ExitCode {
    // Parse command line arguments
    let args = Args::parse();
    // Check for single instance
    let instance = SingleInstance::new(APP_ID).unwrap();
    if !instance.is_single() {
        println!("Another instance of the translator is already running!");
        // TODO: Implement IPC to send focus request to existing instance
        return glib::ExitCode::FAILURE;
    }
    
    // Load environment variables from .env file if present
    dotenv().ok(); // This is still useful for API keys, etc.

    // Load configuration from file (or defaults if not found/invalid)
    let mut config = config::load_config();
    
    // Override debug setting with command line argument if provided
    if args.debug {
        config.debug = true;
    }

    // Create a new application
    // Use gio::ApplicationFlags::HANDLES_COMMAND_LINE to prevent GTK from parsing args
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    // Clone the config to move into the closure
    let initial_config = config.clone();

    // Connect to "activate" signal of `app`
    // Pass the loaded initial config to the UI builder using a closure
    app.connect_activate(move |app| {
        ui::build_ui(app, initial_config.clone()); // Pass the config
    });
    
    // Handle command line to prevent GTK from complaining about unknown options
    app.connect_command_line(|app, _| {
        app.activate();
        0 // Return success
    });

    // Run the application (instance will be dropped when we exit)
    // Instance is automatically dropped here, releasing the lock
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

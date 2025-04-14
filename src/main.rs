// Declare modules
mod language;
mod settings;
mod translation;
mod ui;

use gtk::prelude::*;
use gtk::{glib, Application};
use dotenvy::dotenv;

const APP_ID: &str = "org.gtk_rs.ClipboardTranslator";

// Use tokio runtime for async operations
#[tokio::main]
async fn main() -> glib::ExitCode {
    // Load environment variables from .env file if present
    dotenv().ok();

    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    // Use the build_ui function from the ui module
    app.connect_activate(ui::build_ui);

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

use gtk::prelude::*;
use gtk::{glib, gdk, Application, ApplicationWindow, Label, Button, ToggleButton, Box as GtkBox, Orientation, Align};
use std::rc::Rc;
use std::cell::RefCell;
use std::env;
use tokio::time::{timeout, Duration};
// Use lingua::Language directly
use lingua::{LanguageDetectorBuilder, Language};

use crate::config::Config; // Import Config struct
use crate::settings; // Import settings module
use crate::translation::request_translation;
use crate::clone; // Import the clone macro

/// Implements the language selection algorithm from README.md
/// 
/// # Arguments
/// 
/// * `source_lang` - The detected source language (Option<Language>)
/// * `primary_lang` - The user's primary language
/// * `secondary_lang` - The user's secondary language
/// * `last_lang` - The last selected target language
/// 
/// # Returns
/// 
/// The selected target language based on the algorithm
pub fn choose_target_language(
    source_lang: Option<Language>,
    primary_lang: Language,
    secondary_lang: Language,
    last_lang: Language
) -> Language {
    // 1. If the source isn't the primary language, translate into the primary language
    let is_source_primary = source_lang
        .map(|detected| detected == primary_lang)
        .unwrap_or(false);
    
    if !is_source_primary {
        // Rule 1: If source isn't primary language, translate to primary
        primary_lang
    } else {
        // Source IS primary language
        // Rule 2: If there's a meaningful last choice, use it
        if last_lang != primary_lang {
            last_lang
        } else {
            // Rule 3: Fall back to secondary language
            secondary_lang
        }
    }
}

// --- Helper function to update button states ---
// Now accepts lingua::Language and a slice of button tuples with Language
fn update_active_button_simple(
    active_lang: Language,
    buttons: &[(Language, Rc<RefCell<ToggleButton>>)],
) {
    for (lang, button_rc) in buttons {
        // Compare lingua::Language directly
        button_rc.borrow().set_active(*lang == active_lang);
    }
}


// Modified function signature to accept initial Config
pub fn build_ui(app: &Application, initial_config: Config) {
    // --- State Management ---
    // Use the initial config passed from main
    let config_rc = Rc::new(RefCell::new(initial_config));

    // Load last target language (now lingua::Language) from settings
    let last_target_language = settings::load_last_language();
    let original_clipboard_text = Rc::new(RefCell::new(None::<String>));
    let api_key_rc = Rc::new(RefCell::new(None::<String>)); // Keep API key separate

    // --- Lingua Detector ---
    // Only load languages we need for detection from config
    let detector = {
        let config = config_rc.borrow();
        let detection_languages = vec![
            config.primary_language,
        ];
        
        println!("Setting up language detector with: {:?}", detection_languages);
        Rc::new(LanguageDetectorBuilder::from_languages(&detection_languages).with_low_accuracy_mode().build())
    };


    // --- UI Elements ---

    // Main vertical box
    let main_vbox = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(15) // Increased spacing a bit
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Horizontal box for language buttons
    let lang_hbox = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .halign(Align::Center) // Center the buttons horizontally
        .build();

    // --- Create Language Buttons Dynamically ---
    // Store buttons in a Vec with lingua::Language
    let language_buttons_rc: Rc<RefCell<Vec<(Language, Rc<RefCell<ToggleButton>>)>>> = Rc::new(RefCell::new(Vec::new()));
    { // Scope for borrowing config_rc and language_buttons_rc mutably
        let mut buttons_mut = language_buttons_rc.borrow_mut();
        let config = config_rc.borrow(); // Borrow immutably to read all_target_languages

        if config.all_target_languages.is_empty() {
             // Handle case where config might somehow have an empty list despite defaults
             eprintln!("Error: No target languages defined in configuration!");
             // Maybe add a fallback label here?
        } else {
            for lang in &config.all_target_languages {
                // Get the ISO code and convert to string
                let button_label = lang.iso_code_639_1().to_string().to_uppercase();

                let button = ToggleButton::with_label(&button_label);
                button.set_tooltip_text(Some(&lang.to_string())); // Tooltip shows full name
                lang_hbox.append(&button); // Add button to the UI layout
                buttons_mut.push((*lang, Rc::new(RefCell::new(button)))); // Store lang and button Rc
            }
        }
    } // Mutable borrow of language_buttons_rc drops here


    // Vertical box for content (label + copy button)
    let content_vbox = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();

    // Label for translation output
    let label = Label::builder()
        .label("Reading clipboard...")
        .wrap(true)
        .selectable(true)
        .build();

    // Copy & Close button (standard button)
    let copy_button = Button::with_label("Copy & Close");

    content_vbox.append(&label);
    content_vbox.append(&copy_button);

    // Add language buttons and content box to the main box
    main_vbox.append(&lang_hbox);
    main_vbox.append(&content_vbox);


    // --- Initial Load & Translation ---
    let display = gdk::Display::default().expect("Could not get default display");
    let clipboard = display.clipboard();

    // Clone state Rcs for the initial load future
    let label_clone_init = label.clone();
    let original_text_rc_clone_init = original_clipboard_text.clone();
    let api_key_rc_clone_init = api_key_rc.clone();
    let config_rc_clone_init = config_rc.clone(); // Clone the config Rc
    let detector_clone_init = detector.clone(); // Clone detector for the async block
    let language_buttons_rc_clone_init = language_buttons_rc.clone(); // Clone buttons Vec Rc


    glib::spawn_future_local(async move {
        // 1. Read API Key once (still reading from env var for now)
        match env::var("OPENROUTER_API_KEY") {
            Ok(key) => {
                *api_key_rc_clone_init.borrow_mut() = Some(key);
            }
            Err(_) => {
                label_clone_init.set_text("Error: OPENROUTER_API_KEY environment variable not set.");
                // Update button state even on error (show last language from settings)
                let lang_to_show = last_target_language; // Use last_target_language (lingua::Language) from settings
                // Use the imported clone macro
                glib::idle_add_local_once(clone!(@strong language_buttons_rc_clone_init => move || {
                    update_active_button_simple(lang_to_show, &language_buttons_rc_clone_init.borrow());
                }));
                return; // Stop if no API key
            }
        }

        // 2. Read text from clipboard once
        match clipboard.read_text_future().await {
            Ok(Some(gstring_text)) => { // text is glib::GString here
                let text = gstring_text.to_string(); // Convert to String
                *original_text_rc_clone_init.borrow_mut() = Some(text.clone()); // Store original text as String

                // --- Performance Logging Start ---
                let start_time = std::time::Instant::now();
                println!("Starting language detection at {:?}", start_time);
                
                // --- Language Detection with Timeout ---
                // detected_language is Option<lingua::Language>
                println!("Text length for detection: {} characters", text.len());
                let detection_start = std::time::Instant::now();
                
                // Only use a small sample of text for detection (first 100 chars or less)
                // Use a safe way to truncate that respects UTF-8 character boundaries
                let sample_text = if text.chars().count() > 100 {
                    // Collect first 100 characters safely
                    text.chars().take(100).collect::<String>()
                } else {
                    text.clone()
                };
                
                // Add timeout to prevent long detection times
                let detected_source_lang = match timeout(
                    Duration::from_secs(2), // 2 second timeout
                    async {
                        detector_clone_init.detect_language_of(sample_text)
                    }
                ).await {
                    Ok(lang) => lang,
                    Err(_) => {
                        println!("Language detection timed out after 2 seconds");
                        None // Return None if detection times out
                    }
                };
                
                let detection_duration = detection_start.elapsed();
                println!("Language detection took: {:?}", detection_duration);

                if let Some(lang) = detected_source_lang {
                    println!("Detected source language: {:?}", lang); // Log detected language
                    println!("Total time from start to detection: {:?}", start_time.elapsed());
                } else {
                    println!("Could not detect source language.");
                }

                // --- Implement language selection logic from README.md ---
                let (primary_lang, secondary_lang) = {
                    let config = config_rc_clone_init.borrow();
                    (config.primary_language, config.secondary_language)
                };
                
                // Use the extracted function for language selection
                let mut final_target_lang = choose_target_language(
                    detected_source_lang,
                    primary_lang,
                    secondary_lang,
                    last_target_language
                );
                
                // Log the decision
                match detected_source_lang {
                    Some(src) if src != primary_lang => {
                        println!("Source is not primary language -> Translating to primary ({:?})", primary_lang);
                    },
                    Some(_) => {
                        if last_target_language != primary_lang {
                            println!("Source is primary language and last target ({:?}) is meaningful -> Using last target", last_target_language);
                        } else {
                            println!("Source is primary language and no meaningful last target -> Using secondary ({:?})", secondary_lang);
                        }
                    },
                    None => {
                        println!("Could not detect source language -> Using primary language ({:?})", primary_lang);
                    }
                }

                // Ensure the final_target_lang is actually available in the UI buttons
                let is_target_available = config_rc_clone_init.borrow().all_target_languages.contains(&final_target_lang);
                if !is_target_available {
                    println!("Warning: Auto-selected target language {:?} is not in 'all_target_languages'. Reverting to last target {:?}", final_target_lang, last_target_language);
                    final_target_lang = last_target_language; // Revert if not available
                    // Also ensure the last_target_language itself is available, otherwise pick the first from config?
                    if !config_rc_clone_init.borrow().all_target_languages.contains(&final_target_lang) {
                        println!("Warning: Last target language {:?} is also not in 'all_target_languages'. Using first available.", final_target_lang);
                        final_target_lang = config_rc_clone_init.borrow().all_target_languages.first().cloned().unwrap_or(Language::English); // Fallback to English if list is somehow empty
                    }
                }


                // Update last_target_language in settings if the target language changed
                if final_target_lang != last_target_language {
                    if let Err(e) = settings::save_last_language(final_target_lang) {
                        eprintln!("Failed to save last language after auto-switch: {}", e);
                    } else {
                        println!("Target language automatically set to: {:?} and saved.", final_target_lang);
                    }
                } else {
                    println!("Target language remains: {:?}", final_target_lang);
                }

                // Update buttons in the main thread (always run this to set initial state correctly based on final_target_lang)
                glib::idle_add_local_once(clone!(@strong language_buttons_rc_clone_init => move || {
                    update_active_button_simple(final_target_lang, &language_buttons_rc_clone_init.borrow());
                }));


                // 3. Perform translation with the determined final language
                let (api_url, model_version) = {
                    let config = config_rc_clone_init.borrow();
                    (config.api_url.clone(), config.model_version.clone())
                };

                if let Some(key) = api_key_rc_clone_init.borrow().as_ref() {
                     request_translation(
                         text,
                         final_target_lang, // Use the determined target language (lingua::Language)
                         key.clone(),
                         api_url,
                         model_version,
                         label_clone_init
                     ).await;
                } else {
                     label_clone_init.set_text("Error retrieving API key for translation.");
                }
            }
            Ok(None) => {
                label_clone_init.set_text("Clipboard does not contain text.");
                *original_text_rc_clone_init.borrow_mut() = None; // Ensure it's None
                // Update button state even if clipboard is empty
                let lang_to_show = last_target_language; // Use last_target_language from settings
                 glib::idle_add_local_once(clone!(@strong language_buttons_rc_clone_init => move || {
                    update_active_button_simple(lang_to_show, &language_buttons_rc_clone_init.borrow());
                }));
            }
            Err(e) => {
                eprintln!("Error reading clipboard: {}", e);
                label_clone_init.set_text(&format!("Error reading clipboard: {}", e));
                *original_text_rc_clone_init.borrow_mut() = None; // Ensure it's None
                 // Update button state even on error
                let lang_to_show = last_target_language; // Use last_target_language from settings
                 glib::idle_add_local_once(clone!(@strong language_buttons_rc_clone_init => move || {
                    update_active_button_simple(lang_to_show, &language_buttons_rc_clone_init.borrow());
                }));
            }
        }
    });

    // --- Window Setup ---
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Clipboard Translator")
        .child(&main_vbox)
        .default_width(450)
        .default_height(400) // Adjusted default height slightly
        .build();

    // --- Language Button Toggle Handlers ---
    // Define the handler logic once
    let create_lang_button_handler = |
        button_lang: Language, // The language this specific button represents (lingua::Language)
        all_buttons_rc: Rc<RefCell<Vec<(Language, Rc<RefCell<ToggleButton>>)>>> // Rc to the Vec of all buttons
    | {
        // Clone necessary items for the handler closure
        let config_rc_handler = config_rc.clone(); // Clone config Rc
        let text_rc = original_clipboard_text.clone();
        let key_rc = api_key_rc.clone();
        let label_clone = label.clone();
        // Clone the Rc to the button vector for use inside the closure
        let all_buttons_rc_clone = all_buttons_rc.clone();

        move |toggled_button: &ToggleButton| {
            // Check if the button *became* active.
            if toggled_button.is_active() {
                // Get the previously selected language from settings
                let previously_selected_lang = settings::load_last_language();

                // Only trigger if the language actually changed by user click
                if button_lang != previously_selected_lang {
                    // Save the new language to settings
                    if let Err(e) = settings::save_last_language(button_lang) {
                        eprintln!("Failed to save last language after user selection: {}", e);
                    } else {
                        println!("Target language set by user to: {:?} and saved.", button_lang);
                    }

                    // Get API URL and model version from config
                    let (api_url, model_version) = {
                        let config = config_rc_handler.borrow();
                        (config.api_url.clone(), config.model_version.clone())
                    };

                    // Deactivate other buttons (visually)
                    let all_buttons = all_buttons_rc_clone.borrow(); // Borrow immutably
                    for (lang, other_btn_rc) in all_buttons.iter() {
                        if *lang != button_lang && other_btn_rc.borrow().is_active() {
                            other_btn_rc.borrow().set_active(false);
                        }
                    }
                    // Ensure the clicked button remains active (might be redundant but safe)
                    if !toggled_button.is_active() {
                         toggled_button.set_active(true);
                    }


                    // Get stored text and key
                    let maybe_text = text_rc.borrow().clone();
                    let maybe_key = key_rc.borrow().clone();

                    if let (Some(text), Some(key)) = (maybe_text, maybe_key) {
                         // Spawn a new future for the translation request
                         glib::spawn_future_local(request_translation(
                             text,
                             button_lang, // Use newly set language (lingua::Language)
                             key,
                             api_url,
                             model_version,
                             label_clone.clone(),
                         ));
                    } else {
                         println!("No original text or API key available to translate.");
                         label_clone.set_text("Cannot translate: Missing original text or API key.");
                    }
                } else {
                    // This handles the case where the button was already active (e.g., set by initial load or auto-switch)
                    // and the user clicks it again. We still need to ensure other buttons are off.
                    let all_buttons = all_buttons_rc_clone.borrow();
                    for (lang, other_btn_rc) in all_buttons.iter() {
                        if *lang != button_lang && other_btn_rc.borrow().is_active() {
                            other_btn_rc.borrow().set_active(false);
                        }
                    }
                     // Ensure the clicked button *is* active if it wasn't already
                    if !toggled_button.is_active() {
                         toggled_button.set_active(true);
                    }
                }
            } else {
                // This block handles the case where the user tries to deactivate the *currently active* button.
                // We want to prevent this, ensuring one button is always selected.
                 if button_lang == settings::load_last_language() {
                     // Find the Rc for *this* button to re-activate it
                     let maybe_button_rc = all_buttons_rc_clone.borrow().iter()
                         .find(|(lang, _)| *lang == button_lang)
                         .map(|(_, rc)| rc.clone());

                     if let Some(button_rc_to_reactivate) = maybe_button_rc {
                         // Re-activate the button in the next idle loop iteration.
                         // Using idle_add_local_once prevents potential infinite loops if the signal triggers itself immediately.
                         glib::idle_add_local_once(clone!(@strong button_rc_to_reactivate => move || {
                            // Check again before setting, in case state changed rapidly
                            if !button_rc_to_reactivate.borrow().is_active() {
                                button_rc_to_reactivate.borrow().set_active(true);
                            }
                         }));
                     }
                 }
            }
        }
    };

    // Connect the handler to each button
    { // Scope for borrowing language_buttons_rc
        let buttons = language_buttons_rc.borrow();
        for (lang, button_rc) in buttons.iter() {
            button_rc.borrow().connect_toggled(
                // Create a unique handler closure for each button
                create_lang_button_handler(*lang, language_buttons_rc.clone())
            );
        }
    } // Borrow drops here


    // --- Copy Button Click Handler Setup ---
    let label_clone_copy = label.clone();
    let window_clone_copy = window.clone();
    let clipboard_copy = display.clipboard();

    copy_button.connect_clicked(move |_button| {
        let text_to_copy = label_clone_copy.text();
        clipboard_copy.set_text(&text_to_copy);
        println!("Copied to clipboard and closing: {}", text_to_copy);
        window_clone_copy.close();
    });

    // Present window
    window.present();
}

use gtk::prelude::*;
use gtk::{glib, gdk, Application, ApplicationWindow, Label, Button, ToggleButton, Box as GtkBox, Orientation, Align};
use std::rc::Rc;
use std::cell::RefCell;
use std::env;
use lingua::{LanguageDetectorBuilder, Language};

use crate::language::TargetLanguage;
use crate::settings::{load_language_setting, save_language_setting};
use crate::translation::request_translation;
use crate::clone; // Import the clone macro from main.rs or lib.rs where it's defined

// --- Helper function to update button states (Simplified) ---
fn update_active_button_simple(
    active_lang: TargetLanguage,
    pt_button: &Rc<RefCell<ToggleButton>>,
    en_button: &Rc<RefCell<ToggleButton>>,
    uk_button: &Rc<RefCell<ToggleButton>>,
    ru_button: &Rc<RefCell<ToggleButton>>,
) {
    // Directly set the state. The handlers check `is_active` so they won't loop.
    pt_button.borrow().set_active(active_lang == TargetLanguage::Portuguese);
    en_button.borrow().set_active(active_lang == TargetLanguage::English);
    uk_button.borrow().set_active(active_lang == TargetLanguage::Ukrainian);
    ru_button.borrow().set_active(active_lang == TargetLanguage::Russian);
}


pub fn build_ui(app: &Application) {
    // --- State Management ---
    // Load initial language or default to English
    let initial_language = load_language_setting().unwrap_or(TargetLanguage::English);
    let current_language = Rc::new(RefCell::new(initial_language));
    let original_clipboard_text = Rc::new(RefCell::new(None::<String>));
    let api_key_rc = Rc::new(RefCell::new(None::<String>));

    // --- Lingua Detector ---
    // Create the language detector. Consider creating it once if performance is critical.
    // Preload only the languages we might detect or care about.
    let detector = Rc::new(LanguageDetectorBuilder::from_languages(&[
        Language::English,
        Language::Russian,
        Language::Portuguese,
        Language::Ukrainian,
        // Add other potential source languages if needed
    ]).build());


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

    // Create language toggle buttons and store in Rc<RefCell> for later access
    let pt_button_rc = Rc::new(RefCell::new(ToggleButton::with_label(TargetLanguage::Portuguese.code())));
    let en_button_rc = Rc::new(RefCell::new(ToggleButton::with_label(TargetLanguage::English.code())));
    let uk_button_rc = Rc::new(RefCell::new(ToggleButton::with_label(TargetLanguage::Ukrainian.code())));
    let ru_button_rc = Rc::new(RefCell::new(ToggleButton::with_label(TargetLanguage::Russian.code()))); // Добавлена кнопка RU

    // Add buttons to the hbox
    lang_hbox.append(&*pt_button_rc.borrow());
    lang_hbox.append(&*en_button_rc.borrow());
    lang_hbox.append(&*uk_button_rc.borrow());
    lang_hbox.append(&*ru_button_rc.borrow()); // Добавлена кнопка RU

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
    let current_lang_rc_clone_init = current_language.clone(); // Holds loaded/default lang
    let detector_clone_init = detector.clone(); // Clone detector for the async block

    // Clone buttons for the async block to update UI state
    let pt_button_clone_init = pt_button_rc.clone();
    let en_button_clone_init = en_button_rc.clone();
    let uk_button_clone_init = uk_button_rc.clone();
    let ru_button_clone_init = ru_button_rc.clone();


    glib::spawn_future_local(async move {
        // 1. Read API Key once
        match env::var("OPENROUTER_API_KEY") {
            Ok(key) => {
                *api_key_rc_clone_init.borrow_mut() = Some(key);
            }
            Err(_) => {
                label_clone_init.set_text("Error: OPENROUTER_API_KEY environment variable not set.");
                // Update button state even on error (show default/loaded)
                let lang_to_show = *current_lang_rc_clone_init.borrow();
                // Use the imported clone macro
                glib::idle_add_local_once(clone!(
                    pt_button_clone_init,
                    en_button_clone_init,
                    uk_button_clone_init,
                    ru_button_clone_init
                    => move || {
                        update_active_button_simple( // Use the simpler update function
                            lang_to_show,
                            &pt_button_clone_init,
                            &en_button_clone_init,
                            &uk_button_clone_init,
                            &ru_button_clone_init,
                        );
                    }
                ));
                return; // Stop if no API key
            }
        }

        // 2. Read text from clipboard once
        match clipboard.read_text_future().await {
            Ok(Some(gstring_text)) => { // text is glib::GString here
                let text = gstring_text.to_string(); // Convert to String
                *original_text_rc_clone_init.borrow_mut() = Some(text.clone()); // Store original text as String

                // --- Language Detection ---
                let detected_language: Option<Language> = detector_clone_init.detect_language_of(&text);
                let detected_source_lang = detected_language.and_then(TargetLanguage::from_lingua); // Convert to our enum

                if let Some(lang) = detected_language {
                    println!("Detected source language: {:?}", lang); // Simplified log
                } else {
                    println!("Could not detect source language.");
                }

                // --- Automatic Language Switching Logic ---
                let current_target_lang = *current_lang_rc_clone_init.borrow();
                let mut final_target_lang = current_target_lang; // Start with current

                match detected_source_lang {
                    Some(TargetLanguage::Russian) => {
                        // Source is Russian
                        if current_target_lang == TargetLanguage::Russian {
                            // Case 1: Source is Russian AND Target is Russian -> Target becomes English
                            final_target_lang = TargetLanguage::English;
                            println!("Source Russian, Target Russian -> Switching target to EN");
                        } else {
                            // Case 3 (Implicit): Source is Russian, Target is not Russian -> Keep target
                            println!("Source Russian, Target not Russian -> Keeping target {:?}", final_target_lang);
                            // No change needed: final_target_lang already holds current_target_lang
                        }
                    }
                    Some(detected_lang) => {
                        // Source is NOT Russian (handled above)
                        if detected_lang == current_target_lang {
                             // Case 2: Source is not Russian AND Source == Target -> Target becomes Russian
                            final_target_lang = TargetLanguage::Russian;
                            println!("Source ({:?}) matches Target ({:?}) -> Switching target to RU", detected_lang, current_target_lang);
                        } else {
                            // Case 3 (Implicit): Source is not Russian, Source != Target -> Keep target
                            println!("Source ({:?}) doesn't match Target ({:?}) -> Keeping target {:?}", detected_lang, current_target_lang, final_target_lang);
                            // No change needed
                        }
                    }
                    None => {
                        // Case 3 (Implicit): Could not detect source language -> Keep target
                        println!("Could not detect source language -> Keeping target {:?}", final_target_lang);
                        // No change needed
                    }
                }


                // Update state and UI if language changed or just to set initial state
                if final_target_lang != current_target_lang {
                    *current_lang_rc_clone_init.borrow_mut() = final_target_lang;
                    save_language_setting(final_target_lang);
                    println!("Target language automatically changed to: {:?}", final_target_lang);
                } else {
                    println!("Target language remains: {:?}", final_target_lang);
                }

                // Update buttons in the main thread (always run this to set initial state correctly)
                // Use the imported clone macro
                glib::idle_add_local_once(clone!(
                    pt_button_clone_init,
                    en_button_clone_init,
                    uk_button_clone_init,
                    ru_button_clone_init
                    => move || {
                        update_active_button_simple( // Use the simpler update function
                            final_target_lang, // Use the potentially updated language
                            &pt_button_clone_init,
                            &en_button_clone_init,
                            &uk_button_clone_init,
                            &ru_button_clone_init,
                        );
                    }
                ));


                // 3. Perform translation with the determined final language
                if let Some(key) = api_key_rc_clone_init.borrow().as_ref() {
                     request_translation(text, final_target_lang, key.clone(), label_clone_init).await;
                } else {
                     // Should not happen if API key check passed, but handle defensively
                     label_clone_init.set_text("Error retrieving API key for translation.");
                }
            }
            Ok(None) => {
                label_clone_init.set_text("Clipboard does not contain text.");
                *original_text_rc_clone_init.borrow_mut() = None; // Ensure it's None
                // Update button state even if clipboard is empty
                let lang_to_show = *current_lang_rc_clone_init.borrow();
                 // Use the imported clone macro
                 glib::idle_add_local_once(clone!(
                    pt_button_clone_init,
                    en_button_clone_init,
                    uk_button_clone_init,
                    ru_button_clone_init
                    => move || {
                        update_active_button_simple( // Use the simpler update function
                            lang_to_show,
                            &pt_button_clone_init,
                            &en_button_clone_init,
                            &uk_button_clone_init,
                            &ru_button_clone_init,
                        );
                    }
                ));
            }
            Err(e) => {
                eprintln!("Error reading clipboard: {}", e);
                label_clone_init.set_text(&format!("Error reading clipboard: {}", e));
                *original_text_rc_clone_init.borrow_mut() = None; // Ensure it's None
                 // Update button state even on error
                let lang_to_show = *current_lang_rc_clone_init.borrow();
                 // Use the imported clone macro
                 glib::idle_add_local_once(clone!(
                    pt_button_clone_init,
                    en_button_clone_init,
                    uk_button_clone_init,
                    ru_button_clone_init
                    => move || {
                        update_active_button_simple( // Use the simpler update function
                            lang_to_show,
                            &pt_button_clone_init,
                            &en_button_clone_init,
                            &uk_button_clone_init,
                            &ru_button_clone_init,
                        );
                    }
                ));
            }
        }
    });

    // --- Window Setup ---
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Clipboard Translator")
        .child(&main_vbox)
        .default_width(450)
        .default_height(400)
        .build();

    // --- Language Button Toggle Handlers ---
    let create_lang_button_handler = |
        button_lang: TargetLanguage,
        button_rc: Rc<RefCell<ToggleButton>>,
        other_buttons: Vec<Rc<RefCell<ToggleButton>>>
    | {
        // Clone necessary items for the handler closure
        let lang_rc = current_language.clone();
        let text_rc = original_clipboard_text.clone();
        let key_rc = api_key_rc.clone();
        let label_clone = label.clone();

        move |toggled_button: &ToggleButton| {
            // Check if the button *became* active. Ignore deactivation events handled below.
            if toggled_button.is_active() {
                let previously_selected_lang = *lang_rc.borrow();

                // Only trigger if the language actually changed by user click
                if button_lang != previously_selected_lang {
                    *lang_rc.borrow_mut() = button_lang; // Update current language state

                    // Deactivate other buttons (visually)
                    for other_btn_rc in &other_buttons {
                         // Check if the other button is currently active before deactivating
                         if other_btn_rc.borrow().is_active() {
                            other_btn_rc.borrow().set_active(false);
                         }
                    }
                    // Ensure the clicked button remains active (might be redundant but safe)
                    if !button_rc.borrow().is_active() {
                        button_rc.borrow().set_active(true);
                    }


                    // Save the new setting
                    save_language_setting(button_lang);
                    println!("Target language set by user to: {:?}", button_lang); // Log user change

                    // Get stored text and key
                    let maybe_text = text_rc.borrow().clone();
                    let maybe_key = key_rc.borrow().clone();

                    if let (Some(text), Some(key)) = (maybe_text, maybe_key) {
                         // Spawn a new future for the translation request
                         glib::spawn_future_local(request_translation(
                             text,
                             button_lang, // Use newly set language
                             key,
                             label_clone.clone(),
                         ));
                    } else {
                         println!("No original text or API key available to translate.");
                         label_clone.set_text("Cannot translate: Missing original text or API key.");
                    }
                }
            } else {
                // This block handles the case where the user tries to deactivate the *currently active* button.
                // We want to prevent this, ensuring one button is always selected.
                 if button_lang == *lang_rc.borrow() {
                     // Re-activate the button in the next idle loop iteration.
                     // Use the imported clone macro
                     glib::idle_add_local_once(clone!(@strong button_rc => move || {
                        // Check again before setting, in case state changed rapidly
                        if !button_rc.borrow().is_active() {
                            button_rc.borrow().set_active(true);
                        }
                     }));
                 }
            }
        }
    };

    // Setup handlers, passing the other buttons to deactivate
    pt_button_rc.borrow().connect_toggled(create_lang_button_handler(
        TargetLanguage::Portuguese,
        pt_button_rc.clone(),
        vec![en_button_rc.clone(), uk_button_rc.clone(), ru_button_rc.clone()]
    ));
    en_button_rc.borrow().connect_toggled(create_lang_button_handler(
        TargetLanguage::English,
        en_button_rc.clone(),
        vec![pt_button_rc.clone(), uk_button_rc.clone(), ru_button_rc.clone()]
    ));
    uk_button_rc.borrow().connect_toggled(create_lang_button_handler(
        TargetLanguage::Ukrainian,
        uk_button_rc.clone(),
        vec![pt_button_rc.clone(), en_button_rc.clone(), ru_button_rc.clone()]
    ));
    ru_button_rc.borrow().connect_toggled(create_lang_button_handler(
        TargetLanguage::Russian,
        ru_button_rc.clone(),
        vec![pt_button_rc.clone(), en_button_rc.clone(), uk_button_rc.clone()]
    ));


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

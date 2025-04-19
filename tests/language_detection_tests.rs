use lingua::{Language, LanguageDetectorBuilder};

#[test]
fn test_language_detection() {
    // Create a detector with a subset of languages
    let languages = vec![
        Language::English,
        Language::French,
        Language::German,
        Language::Spanish,
        Language::Italian
    ];
    
    let detector = LanguageDetectorBuilder::from_languages(&languages).build();
    
    // Test cases with expected languages
    let test_cases = vec![
        ("Hello world, this is a test of the language detection system.", Language::English),
        ("Bonjour le monde, ceci est un test du système de détection de langue.", Language::French),
        ("Hallo Welt, dies ist ein Test des Spracherkennungssystems.", Language::German),
        ("Hola mundo, esta es una prueba del sistema de detección de idiomas.", Language::Spanish),
        ("Ciao mondo, questo è un test del sistema di rilevamento della lingua.", Language::Italian),
    ];
    
    for (text, expected_lang) in test_cases {
        let detected = detector.detect_language_of(text);
        assert_eq!(detected, Some(expected_lang), "Failed to detect correct language for: {}", text);
    }
}

#[test]
fn test_language_detection_with_short_text() {
    // Create a detector with a subset of languages
    let languages = vec![
        Language::English,
        Language::French,
        Language::German,
        Language::Spanish,
        Language::Italian
    ];
    
    let detector = LanguageDetectorBuilder::from_languages(&languages).build();
    
    // Test with very short text (may be less reliable)
    let short_text = "Hello";
    let detected = detector.detect_language_of(short_text);
    
    // We can't be 100% sure of detection with very short text,
    // but it should return something and not panic
    assert!(detected.is_some(), "Should return some language even for short text");
}

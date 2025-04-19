use lingua::Language;

// Import the function that implements the language selection algorithm
use translator::ui::choose_target_language;

#[test]
fn test_language_selection_algorithm() {
    // Test cases based on the algorithm in README.md
    
    // Case 1: Source is not primary language -> translate to primary
    let result = choose_target_language(
        Some(Language::German),  // Source language
        Language::English,       // Primary language
        Language::French,        // Secondary language
        Language::Spanish        // Last language
    );
    assert_eq!(result, Language::English, "Should translate to primary when source is not primary");
    
    // Case 2: Source is primary language and there's a meaningful last choice
    let result = choose_target_language(
        Some(Language::English), // Source language (same as primary)
        Language::English,       // Primary language
        Language::French,        // Secondary language
        Language::Spanish        // Last language (not primary)
    );
    assert_eq!(result, Language::Spanish, "Should use last language when source is primary and last is meaningful");
    
    // Case 3: Source is primary language and last choice is not meaningful (same as primary)
    let result = choose_target_language(
        Some(Language::English), // Source language (same as primary)
        Language::English,       // Primary language
        Language::French,        // Secondary language
        Language::English        // Last language (same as primary, not meaningful)
    );
    assert_eq!(result, Language::French, "Should use secondary when source is primary and last is not meaningful");
    
    // Case 4: Source language detection failed
    let result = choose_target_language(
        None,                    // Source language detection failed
        Language::English,       // Primary language
        Language::French,        // Secondary language
        Language::Spanish        // Last language
    );
    assert_eq!(result, Language::English, "Should default to primary when source detection fails");
}

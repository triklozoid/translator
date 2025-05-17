use translator::ui::choose_target_language;
use lingua::Language;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_target_language_source_not_primary() {
        // Test case 1: Source is not primary language
        let result = choose_target_language(
            Some(Language::Spanish),
            Language::English,
            Language::French,
            Language::German
        );
        assert_eq!(result, Language::English);
    }

    #[test]
    fn test_choose_target_language_source_is_primary_with_last() {
        // Test case 2: Source is primary language and there's a meaningful last choice
        let result = choose_target_language(
            Some(Language::English),
            Language::English,
            Language::French,
            Language::German
        );
        assert_eq!(result, Language::German);
    }

    #[test]
    fn test_choose_target_language_source_is_primary_no_last() {
        // Test case 3: Source is primary language and last choice is same as primary
        let result = choose_target_language(
            Some(Language::English),
            Language::English,
            Language::French,
            Language::English
        );
        assert_eq!(result, Language::French);
    }

    #[test]
    fn test_choose_target_language_no_source_detected() {
        // Test case 4: No source language detected
        let result = choose_target_language(
            None,
            Language::English,
            Language::French,
            Language::German
        );
        // When source is None, it's not primary, so should return primary
        assert_eq!(result, Language::English);
    }

    #[test]
    fn test_choose_target_language_all_same_language() {
        // Edge case: All languages are the same
        let result = choose_target_language(
            Some(Language::English),
            Language::English,
            Language::English,
            Language::English
        );
        // Should return secondary (even though it's the same)
        assert_eq!(result, Language::English);
    }

    #[test]
    fn test_choose_target_language_different_combinations() {
        // Test various language combinations
        let test_cases = vec![
            // (source, primary, secondary, last, expected)
            (Some(Language::French), Language::English, Language::Spanish, Language::Italian, Language::English),
            (Some(Language::German), Language::German, Language::English, Language::French, Language::French),
            (Some(Language::Italian), Language::Italian, Language::Spanish, Language::Italian, Language::Spanish),
            (None, Language::Russian, Language::Polish, Language::Ukrainian, Language::Russian),
        ];

        for (source, primary, secondary, last, expected) in test_cases {
            let result = choose_target_language(source, primary, secondary, last);
            assert_eq!(result, expected);
        }
    }
}
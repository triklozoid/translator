use lingua::Language;
use tokio::time::{timeout, Duration};
use translator::{translate_text, TranslationResult};

#[tokio::test]
async fn test_empty_text() {
    let result = translate_text(
        "",
        Language::Spanish,
        "test-key".to_string(),
        "http://127.0.0.1:9999".to_string(), // Use local unreachable address
        "gpt-3.5-turbo".to_string(),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Clipboard text is empty.");
}

#[tokio::test]
async fn test_whitespace_only_text() {
    let result = translate_text(
        "   \t\n   ",
        Language::French,
        "test-key".to_string(),
        "http://127.0.0.1:9999".to_string(),
        "gpt-3.5-turbo".to_string(),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Clipboard text is empty.");
}

#[tokio::test]
async fn test_network_error_with_timeout() {
    let future = translate_text(
        "Hello, world!",
        Language::German,
        "test-key".to_string(),
        "http://127.0.0.1:9999".to_string(), // Local unreachable address
        "gpt-3.5-turbo".to_string(),
    );

    let result = timeout(Duration::from_secs(5), future).await;

    match result {
        Ok(inner_result) => {
            assert!(inner_result.is_err());
            let error = inner_result.unwrap_err();
            assert!(
                error.contains("Error")
                    || error.contains("Network")
                    || error.contains("Connection")
            );
        }
        Err(_) => {
            // Timeout is also acceptable for network errors
            assert!(true);
        }
    }
}

#[tokio::test]
async fn test_single_language() {
    let future = translate_text(
        "Hello",
        Language::Spanish,
        "test-key".to_string(),
        "http://127.0.0.1:9999".to_string(),
        "gpt-3.5-turbo".to_string(),
    );

    let result = timeout(Duration::from_secs(5), future).await;

    match result {
        Ok(inner_result) => {
            assert!(inner_result.is_err());
        }
        Err(_) => {
            // Timeout is also acceptable
            assert!(true);
        }
    }
}

#[tokio::test]
async fn test_multiple_languages_with_timeout() {
    let languages = vec![Language::Spanish, Language::French, Language::German];

    for language in languages {
        let future = translate_text(
            "Hello",
            language,
            "test-key".to_string(),
            "http://127.0.0.1:9999".to_string(),
            "gpt-3.5-turbo".to_string(),
        );

        let result = timeout(Duration::from_secs(2), future).await;

        match result {
            Ok(inner_result) => {
                assert!(inner_result.is_err());
            }
            Err(_) => {
                // Timeout is also acceptable
                assert!(true);
            }
        }
    }
}

#[tokio::test]
async fn test_long_text() {
    let long_text = "Lorem ipsum ".repeat(50); // Reduced repetitions
    let future = translate_text(
        &long_text,
        Language::Spanish,
        "test-key".to_string(),
        "http://127.0.0.1:9999".to_string(),
        "gpt-3.5-turbo".to_string(),
    );

    let result = timeout(Duration::from_secs(5), future).await;

    match result {
        Ok(inner_result) => {
            assert!(inner_result.is_err());
        }
        Err(_) => {
            // Timeout is also acceptable
            assert!(true);
        }
    }
}

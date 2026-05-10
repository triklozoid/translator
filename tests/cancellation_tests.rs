use lingua::Language;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};
use translator::translate_text;

const TEST_PROVIDER: &str = "openrouter";

#[tokio::test]
async fn test_translation_cancellation() {
    // Create a cancellation channel
    let (cancel_tx, cancel_rx) = oneshot::channel();
    
    // Start a translation that would take a while (with unreachable URL)
    let translation_future = translate_text(
        "Hello, world!",
        Language::Spanish,
        "test-key".to_string(),
        "http://127.0.0.1:9999".to_string(), // Unreachable URL to simulate long request
        "gpt-3.5-turbo".to_string(),
        TEST_PROVIDER,
        Some(cancel_rx),
        None, // No logger for tests
    );
    
    // Cancel immediately
    let _ = cancel_tx.send(());
    
    // The translation should be cancelled quickly
    let result = timeout(Duration::from_millis(100), translation_future).await;
    
    match result {
        Ok(translation_result) => {
            // Should be an error with cancellation message
            assert!(translation_result.is_err());
            let error = translation_result.unwrap_err();
            assert_eq!(error, "Translation cancelled");
        }
        Err(_) => {
            // Timeout is not expected for cancelled operation
            panic!("Translation was not cancelled within expected time");
        }
    }
}

#[tokio::test]
async fn test_translation_without_cancellation() {
    // Test normal operation without cancellation channel
    let result = translate_text(
        "Hello",
        Language::Spanish,
        "test-key".to_string(),
        "http://127.0.0.1:9999".to_string(),
        "gpt-3.5-turbo".to_string(),
        TEST_PROVIDER,
        None, // No cancellation
        None, // No logger for tests
    )
    .await;
    
    // Should still work normally (though will fail due to unreachable URL)
    assert!(result.is_err());
    let error = result.unwrap_err();
    // Should not be a cancellation error
    assert_ne!(error, "Translation cancelled");
}

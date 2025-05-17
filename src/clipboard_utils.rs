use gtk::gdk;

pub struct ClipboardError {
    pub message: String,
}

impl From<String> for ClipboardError {
    fn from(msg: String) -> Self {
        ClipboardError { message: msg }
    }
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::fmt::Debug for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClipboardError({})", self.message)
    }
}

impl std::error::Error for ClipboardError {}

pub async fn read_clipboard_text(clipboard: &gdk::Clipboard) -> Result<String, ClipboardError> {
    let text_future = clipboard.read_text_future();
    match text_future.await {
        Ok(Some(text)) => Ok(text.to_string()),
        Ok(None) => Err(ClipboardError::from("Clipboard text is empty.".to_string())),
        Err(e) => Err(ClipboardError::from(format!(
            "Failed to read from clipboard: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_clipboard_error_display() {
        let error = ClipboardError::from("Test error".to_string());
        assert_eq!(format!("{}", error), "Test error");
    }

    #[test]
    fn test_clipboard_error_debug() {
        let error = ClipboardError::from("Test error".to_string());
        assert_eq!(format!("{:?}", error), "ClipboardError(Test error)");
    }

    #[test]
    fn test_clipboard_error_trait() {
        let error = ClipboardError::from("Test error".to_string());
        // Test that it implements Error trait
        let _: &dyn Error = &error;
    }
}

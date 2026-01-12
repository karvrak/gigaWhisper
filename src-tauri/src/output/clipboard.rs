//! Clipboard Operations
//!
//! Read/write clipboard content.

use arboard::Clipboard;

/// Clipboard errors
#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("Failed to access clipboard: {0}")]
    Access(String),

    #[error("Failed to get clipboard content: {0}")]
    Get(String),

    #[error("Failed to set clipboard content: {0}")]
    Set(String),
}

/// Get current clipboard text
pub fn get_text() -> Result<String, ClipboardError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| ClipboardError::Access(e.to_string()))?;

    clipboard
        .get_text()
        .map_err(|e| ClipboardError::Get(e.to_string()))
}

/// Set clipboard text
pub fn set_text(text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| ClipboardError::Access(e.to_string()))?;

    clipboard
        .set_text(text)
        .map_err(|e| ClipboardError::Set(e.to_string()))
}

/// Copy text to clipboard (alias for set_text)
pub fn copy_to_clipboard(text: &str) -> Result<(), ClipboardError> {
    set_text(text)
}

/// Paste text using clipboard and Ctrl+V
/// Preserves original clipboard content
pub async fn paste_text(text: &str) -> Result<(), ClipboardError> {
    use super::keyboard;

    // Save current clipboard
    let previous = get_text().ok();

    // Set new text
    set_text(text)?;

    // Small delay to ensure clipboard is set
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Simulate paste
    keyboard::send_ctrl_v()
        .map_err(|e| ClipboardError::Set(e.to_string()))?;

    // Wait for paste to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Restore previous clipboard content
    if let Some(prev) = previous {
        let _ = set_text(&prev);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_roundtrip() {
        // This test may fail on CI without display
        let text = "GigaWhisper test";

        if set_text(text).is_ok() {
            let result = get_text();
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), text);
        }
    }
}

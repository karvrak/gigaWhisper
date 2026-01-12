//! Clipboard Commands
//!
//! Handle text output and history operations.

use tauri::State;
use crate::AppState;

/// Paste text to the active application
#[tauri::command]
pub async fn paste_text(_state: State<'_, AppState>, text: String) -> Result<(), String> {
    tracing::info!("Pasting text: {} chars", text.len());

    // TODO: Implement actual paste logic
    // 1. Save current clipboard
    // 2. Set text to clipboard
    // 3. Simulate Ctrl+V
    // 4. Restore clipboard

    // For now, just set clipboard
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;

    clipboard
        .set_text(&text)
        .map_err(|e| format!("Failed to set clipboard: {}", e))?;

    tracing::info!("Text copied to clipboard");
    Ok(())
}

/// Get transcription history
#[tauri::command]
pub fn get_history(_state: State<'_, AppState>) -> Vec<HistoryEntry> {
    // TODO: Load from persistent storage
    Vec::new()
}

/// DTO for history entry
#[derive(serde::Serialize)]
pub struct HistoryEntry {
    pub id: String,
    pub text: String,
    pub timestamp: u64,
    pub duration_ms: u64,
    pub provider: String,
}

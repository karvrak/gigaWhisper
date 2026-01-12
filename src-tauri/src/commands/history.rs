//! History Commands
//!
//! Tauri commands for managing transcription history.

use crate::history::{self, HistoryEntry};
use base64::{engine::general_purpose::STANDARD, Engine};

/// Get all transcription history entries (newest first)
#[tauri::command]
pub fn get_transcription_history() -> Vec<HistoryEntry> {
    let history = history::get_history();
    history.read().entries()
}

/// Get a specific history entry by ID
#[tauri::command]
pub fn get_history_entry(id: String) -> Option<HistoryEntry> {
    let history = history::get_history();
    history.read().get(&id)
}

/// Delete a history entry by ID
#[tauri::command]
pub fn delete_history_entry(id: String) -> bool {
    let history = history::get_history();

    // Get entry to find audio path before deletion
    let audio_path = {
        let h = history.read();
        h.get(&id).and_then(|e| e.audio_path.clone())
    };

    let mut history = history.write();
    let deleted = history.delete(&id);
    if deleted {
        let _ = history.save();

        // Delete audio file if exists
        if let Some(path) = audio_path {
            let _ = std::fs::remove_file(&path);
            tracing::debug!("Deleted audio file: {}", path);
        }
    }
    deleted
}

/// Clear all history
#[tauri::command]
pub fn clear_history() {
    let history = history::get_history();

    // Collect all audio paths before clearing
    let audio_paths: Vec<String> = {
        let h = history.read();
        h.entries()
            .iter()
            .filter_map(|e| e.audio_path.clone())
            .collect()
    };

    let mut history = history.write();
    history.clear();
    let _ = history.save();

    // Delete all audio files
    for path in audio_paths {
        let _ = std::fs::remove_file(&path);
    }

    // Try to remove the audio directory if empty
    let _ = std::fs::remove_dir(crate::history::audio_dir());
}

/// Get history count
#[tauri::command]
pub fn get_history_count() -> usize {
    let history = history::get_history();
    history.read().len()
}

/// Get audio data as base64 for a history entry
#[tauri::command]
pub fn get_audio_data(id: String) -> Result<String, String> {
    let history = history::get_history();
    let entry = history
        .read()
        .get(&id)
        .ok_or_else(|| "Entry not found".to_string())?;

    let audio_path = entry
        .audio_path
        .as_ref()
        .ok_or_else(|| "No audio file for this entry".to_string())?;

    let audio_bytes = std::fs::read(audio_path)
        .map_err(|e| format!("Failed to read audio file: {}", e))?;

    let base64_data = STANDARD.encode(&audio_bytes);

    Ok(format!("data:audio/wav;base64,{}", base64_data))
}

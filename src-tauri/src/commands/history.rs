//! History Commands
//!
//! Tauri commands for managing transcription history.

use crate::history::{self, HistoryEntry};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::Path;

/// Validate that a file path is safely within the audio directory.
/// Returns the canonicalized path if valid, or None if the path is outside
/// the expected audio directory (potential path traversal attack).
fn validate_audio_path(audio_path: &str) -> Option<std::path::PathBuf> {
    let audio_dir = crate::history::audio_dir();

    // Canonicalize the audio directory (create it if needed for canonicalization)
    let _ = std::fs::create_dir_all(&audio_dir);
    let canonical_audio_dir = match std::fs::canonicalize(&audio_dir) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Failed to canonicalize audio directory: {}", e);
            return None;
        }
    };

    // Canonicalize the provided path
    let path = Path::new(audio_path);
    let canonical_path = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Failed to canonicalize audio path '{}': {}", audio_path, e);
            return None;
        }
    };

    // Verify the path is within the audio directory
    if canonical_path.starts_with(&canonical_audio_dir) {
        Some(canonical_path)
    } else {
        tracing::warn!(
            "Path traversal attempt detected: '{}' is not within audio directory '{}'",
            audio_path,
            audio_dir.display()
        );
        None
    }
}

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

        // Delete audio file if exists and path is valid
        if let Some(path) = audio_path {
            if let Some(validated_path) = validate_audio_path(&path) {
                let _ = std::fs::remove_file(&validated_path);
                tracing::debug!("Deleted audio file: {}", validated_path.display());
            }
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

    // Delete all audio files (only if path is valid)
    for path in audio_paths {
        if let Some(validated_path) = validate_audio_path(&path) {
            let _ = std::fs::remove_file(&validated_path);
        }
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

    // Validate the path is within the audio directory (prevent path traversal)
    let validated_path = validate_audio_path(audio_path)
        .ok_or_else(|| "Invalid audio file path".to_string())?;

    let audio_bytes = std::fs::read(&validated_path)
        .map_err(|e| format!("Failed to read audio file: {}", e))?;

    let base64_data = STANDARD.encode(&audio_bytes);

    Ok(format!("data:audio/wav;base64,{}", base64_data))
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Path Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_audio_path_nonexistent() {
        // A non-existent path should return None (canonicalization fails)
        let result = validate_audio_path("/nonexistent/path/audio.wav");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_audio_path_empty_string() {
        let result = validate_audio_path("");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_audio_path_relative_traversal() {
        // Path traversal attempts should fail
        let result = validate_audio_path("../../../etc/passwd");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_audio_path_windows_traversal() {
        // Windows-style path traversal
        let result = validate_audio_path("..\\..\\..\\Windows\\System32\\config\\SAM");
        assert!(result.is_none());
    }

    // =========================================================================
    // get_transcription_history Tests
    // =========================================================================

    #[test]
    fn test_get_transcription_history_returns_vec() {
        // Should return a Vec without panicking
        let history = get_transcription_history();
        // Type check - should be Vec<HistoryEntry>
        let _: Vec<HistoryEntry> = history;
    }

    #[test]
    fn test_get_transcription_history_ordering() {
        // History should be newest first (checked by the implementation)
        let history = get_transcription_history();

        // If there are multiple entries, verify ordering
        if history.len() >= 2 {
            for i in 0..history.len() - 1 {
                // Timestamps should be in descending order (newest first)
                assert!(
                    history[i].timestamp >= history[i + 1].timestamp,
                    "History should be sorted newest first"
                );
            }
        }
    }

    // =========================================================================
    // get_history_entry Tests
    // =========================================================================

    #[test]
    fn test_get_history_entry_nonexistent() {
        // Getting a non-existent entry should return None
        let result = get_history_entry("nonexistent-id-12345".to_string());
        assert!(result.is_none());
    }

    #[test]
    fn test_get_history_entry_empty_id() {
        let result = get_history_entry("".to_string());
        assert!(result.is_none());
    }

    // =========================================================================
    // delete_history_entry Tests
    // =========================================================================

    #[test]
    fn test_delete_history_entry_nonexistent() {
        // Deleting a non-existent entry should return false
        let result = delete_history_entry("nonexistent-id-67890".to_string());
        assert!(!result);
    }

    #[test]
    fn test_delete_history_entry_empty_id() {
        let result = delete_history_entry("".to_string());
        assert!(!result);
    }

    // =========================================================================
    // get_history_count Tests
    // =========================================================================

    #[test]
    fn test_get_history_count_returns_usize() {
        let count = get_history_count();
        // Type check - should be usize
        let _: usize = count;
        // Count should be non-negative (always true for usize)
    }

    // =========================================================================
    // get_audio_data Tests
    // =========================================================================

    #[test]
    fn test_get_audio_data_nonexistent_entry() {
        let result = get_audio_data("nonexistent-id-audio".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_get_audio_data_empty_id() {
        let result = get_audio_data("".to_string());
        assert!(result.is_err());
    }

    // =========================================================================
    // clear_history Tests
    // =========================================================================

    #[test]
    fn test_clear_history_does_not_panic() {
        // clear_history should not panic even if called multiple times
        clear_history();
        clear_history();

        // After clear, count should be 0
        assert_eq!(get_history_count(), 0);
    }

    // =========================================================================
    // Integration-style Tests
    // =========================================================================

    #[test]
    fn test_history_count_matches_entries() {
        let entries = get_transcription_history();
        let count = get_history_count();

        assert_eq!(entries.len(), count);
    }
}

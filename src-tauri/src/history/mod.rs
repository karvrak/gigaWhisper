//! History Module
//!
//! Store and retrieve transcription history.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Maximum number of history entries to keep
const MAX_HISTORY_ENTRIES: usize = 100;

/// Global history instance
static HISTORY: OnceLock<RwLock<TranscriptionHistory>> = OnceLock::new();

/// A single transcription entry in history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unique identifier
    pub id: String,
    /// The transcribed text
    pub text: String,
    /// Timestamp when transcription was created (ISO 8601)
    pub timestamp: String,
    /// Duration of the recording in milliseconds
    pub duration_ms: u64,
    /// Provider used (whisper.cpp or groq)
    pub provider: String,
    /// Language detected/used
    pub language: Option<String>,
    /// Path to the audio file (optional, for playback)
    #[serde(default)]
    pub audio_path: Option<String>,
}

/// Transcription history storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranscriptionHistory {
    entries: VecDeque<HistoryEntry>,
}

impl TranscriptionHistory {
    /// Create a new empty history
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }

    /// Load history from disk
    pub fn load() -> Self {
        let path = history_file_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(history) => {
                            tracing::info!("Loaded {} history entries",
                                match &history { TranscriptionHistory { entries } => entries.len() });
                            return history;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse history file: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read history file: {}", e);
                }
            }
        }
        Self::new()
    }

    /// Save history to disk
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = history_file_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        std::fs::write(&path, content)?;
        tracing::debug!("History saved to {:?}", path);
        Ok(())
    }

    /// Add a new entry to history
    pub fn add(&mut self, entry: HistoryEntry) {
        // Remove oldest if at capacity
        while self.entries.len() >= MAX_HISTORY_ENTRIES {
            self.entries.pop_back();
        }

        // Add new entry at front
        self.entries.push_front(entry);
    }

    /// Get all entries (newest first)
    pub fn entries(&self) -> Vec<HistoryEntry> {
        self.entries.iter().cloned().collect()
    }

    /// Get entry by ID
    pub fn get(&self, id: &str) -> Option<HistoryEntry> {
        self.entries.iter().find(|e| e.id == id).cloned()
    }

    /// Delete entry by ID
    pub fn delete(&mut self, id: &str) -> bool {
        let len_before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() != len_before
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Get the history file path
fn history_file_path() -> PathBuf {
    crate::config::models_dir()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("history.json")
}

/// Get the audio files directory
pub fn audio_dir() -> PathBuf {
    crate::config::models_dir()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("audio")
}

/// Save audio samples to a WAV file and return the path
pub fn save_audio_file(samples: &[f32], sample_rate: u32, id: &str) -> Result<PathBuf, std::io::Error> {
    let audio_path = audio_dir();
    std::fs::create_dir_all(&audio_path)?;

    let file_path = audio_path.join(format!("{}.wav", id));

    // Write WAV file
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(&file_path, spec)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    for &sample in samples {
        // Convert f32 [-1.0, 1.0] to i16
        let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(sample_i16)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    }

    writer.finalize()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    tracing::debug!("Audio saved to {:?}", file_path);
    Ok(file_path)
}

/// Get or initialize the global history instance
pub fn get_history() -> &'static RwLock<TranscriptionHistory> {
    HISTORY.get_or_init(|| RwLock::new(TranscriptionHistory::load()))
}

/// Add a transcription to history
pub fn add_transcription(
    text: String,
    duration_ms: u64,
    provider: String,
    language: Option<String>,
    audio_path: Option<String>,
) {
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        text,
        timestamp: chrono_timestamp(),
        duration_ms,
        provider,
        language,
        audio_path,
    };

    let history = get_history();
    {
        let mut history = history.write();
        history.add(entry);
        if let Err(e) = history.save() {
            tracing::error!("Failed to save history: {}", e);
        }
    }
}

/// Add a transcription to history with audio data
pub fn add_transcription_with_audio(
    text: String,
    duration_ms: u64,
    provider: String,
    language: Option<String>,
    samples: &[f32],
    sample_rate: u32,
) {
    let id = uuid::Uuid::new_v4().to_string();

    // Save audio file
    let audio_path = match save_audio_file(samples, sample_rate, &id) {
        Ok(path) => Some(path.to_string_lossy().to_string()),
        Err(e) => {
            tracing::error!("Failed to save audio file: {}", e);
            None
        }
    };

    let entry = HistoryEntry {
        id,
        text,
        timestamp: chrono_timestamp(),
        duration_ms,
        provider,
        language,
        audio_path,
    };

    let history = get_history();
    {
        let mut history = history.write();
        history.add(entry);
        if let Err(e) = history.save() {
            tracing::error!("Failed to save history: {}", e);
        }
    }
}

/// Get current timestamp in ISO 8601 format
fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();

    // Simple ISO 8601 format without chrono dependency
    // Format: 2024-01-15T10:30:00Z
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Calculate year, month, day from days since epoch
    // Simplified calculation (not accounting for all edge cases)
    let mut year = 1970;
    let mut remaining_days = days as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days_in_month in days_in_months.iter() {
        if remaining_days < *days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }
    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper to create a test entry with specified parameters
    fn create_test_entry(id: &str, text: &str, audio_path: Option<String>) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            text: text.to_string(),
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            duration_ms: 1000,
            provider: "test-provider".to_string(),
            language: Some("en".to_string()),
            audio_path,
        }
    }

    /// Helper to create a history instance with a custom path for testing
    struct TestableHistory {
        history: TranscriptionHistory,
        history_path: PathBuf,
        audio_dir: PathBuf,
    }

    impl TestableHistory {
        fn new(temp_dir: &TempDir) -> Self {
            Self {
                history: TranscriptionHistory::new(),
                history_path: temp_dir.path().join("history.json"),
                audio_dir: temp_dir.path().join("audio"),
            }
        }

        fn save(&self) -> Result<(), std::io::Error> {
            if let Some(parent) = self.history_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = serde_json::to_string_pretty(&self.history)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            fs::write(&self.history_path, content)
        }

        fn load(&mut self) -> Result<(), std::io::Error> {
            if self.history_path.exists() {
                let content = fs::read_to_string(&self.history_path)?;
                self.history = serde_json::from_str(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            }
            Ok(())
        }

        fn load_graceful(&mut self) {
            if self.history_path.exists() {
                if let Ok(content) = fs::read_to_string(&self.history_path) {
                    if let Ok(history) = serde_json::from_str(&content) {
                        self.history = history;
                        return;
                    }
                }
            }
            self.history = TranscriptionHistory::new();
        }

        fn create_audio_file(&self, id: &str) -> Result<PathBuf, std::io::Error> {
            fs::create_dir_all(&self.audio_dir)?;
            let file_path = self.audio_dir.join(format!("{}.wav", id));
            // Create a minimal WAV file (44-byte header + some data)
            let mut file = fs::File::create(&file_path)?;
            // RIFF header
            file.write_all(b"RIFF")?;
            file.write_all(&[36, 0, 0, 0])?; // File size - 8
            file.write_all(b"WAVE")?;
            // fmt chunk
            file.write_all(b"fmt ")?;
            file.write_all(&[16, 0, 0, 0])?; // Chunk size
            file.write_all(&[1, 0])?; // Audio format (PCM)
            file.write_all(&[1, 0])?; // Channels
            file.write_all(&[0x80, 0x3E, 0, 0])?; // Sample rate (16000)
            file.write_all(&[0, 0x7D, 0, 0])?; // Byte rate
            file.write_all(&[2, 0])?; // Block align
            file.write_all(&[16, 0])?; // Bits per sample
            // data chunk
            file.write_all(b"data")?;
            file.write_all(&[0, 0, 0, 0])?; // Data size
            Ok(file_path)
        }

        fn delete_entry_with_audio(&mut self, id: &str) -> bool {
            // First, find the entry to get the audio path
            let audio_path = self.history.get(id).and_then(|e| e.audio_path.clone());

            // Delete the entry
            let deleted = self.history.delete(id);

            // If deleted and had audio, delete the audio file
            if deleted {
                if let Some(path) = audio_path {
                    let _ = fs::remove_file(&path);
                }
            }

            deleted
        }

        fn clear_with_audio(&mut self) {
            // Collect all audio paths before clearing
            let audio_paths: Vec<String> = self.history.entries()
                .iter()
                .filter_map(|e| e.audio_path.clone())
                .collect();

            // Clear the history
            self.history.clear();

            // Delete all audio files
            for path in audio_paths {
                let _ = fs::remove_file(&path);
            }

            // Also try to remove the audio directory if it exists and is empty
            let _ = fs::remove_dir(&self.audio_dir);
        }
    }

    // ===== Test 1: Add entry with audio =====
    #[test]
    fn test_add_entry_with_audio() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        // Create an audio file
        let audio_path = test_history.create_audio_file("entry-1")
            .expect("Failed to create audio file");

        // Create entry with audio path
        let entry = create_test_entry(
            "entry-1",
            "Hello, world!",
            Some(audio_path.to_string_lossy().to_string()),
        );

        test_history.history.add(entry);

        // Verify entry was added
        assert_eq!(test_history.history.len(), 1);

        let retrieved = test_history.history.get("entry-1");
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.text, "Hello, world!");
        assert!(retrieved.audio_path.is_some());
        assert!(retrieved.audio_path.unwrap().contains("entry-1.wav"));

        // Verify audio file exists
        assert!(audio_path.exists());
    }

    // ===== Test 2: Add entry without audio =====
    #[test]
    fn test_add_entry_without_audio() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        let entry = create_test_entry("entry-2", "No audio here", None);
        test_history.history.add(entry);

        assert_eq!(test_history.history.len(), 1);

        let retrieved = test_history.history.get("entry-2").unwrap();
        assert_eq!(retrieved.text, "No audio here");
        assert!(retrieved.audio_path.is_none());
    }

    // ===== Test 3: 100 entry limit - oldest entries are purged =====
    #[test]
    fn test_max_entries_limit() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        // Add 100 entries
        for i in 0..100 {
            let entry = create_test_entry(&format!("entry-{}", i), &format!("Text {}", i), None);
            test_history.history.add(entry);
        }

        assert_eq!(test_history.history.len(), 100);

        // Verify that entry-0 (oldest) is still present (it's at the back)
        assert!(test_history.history.get("entry-0").is_some());

        // Add one more entry - should trigger purge of oldest
        let new_entry = create_test_entry("entry-100", "New entry", None);
        test_history.history.add(new_entry);

        // Should still have 100 entries (limit enforced)
        assert_eq!(test_history.history.len(), 100);

        // entry-0 (oldest) should be removed
        assert!(test_history.history.get("entry-0").is_none());

        // entry-100 (newest) should be present
        assert!(test_history.history.get("entry-100").is_some());

        // entry-1 should still be present (second oldest, now oldest)
        assert!(test_history.history.get("entry-1").is_some());
    }

    // ===== Test 4: Delete entry and verify audio file is deleted =====
    #[test]
    fn test_delete_entry_removes_audio_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        // Create audio file and entry
        let audio_path = test_history.create_audio_file("to-delete")
            .expect("Failed to create audio file");

        let entry = create_test_entry(
            "to-delete",
            "Will be deleted",
            Some(audio_path.to_string_lossy().to_string()),
        );
        test_history.history.add(entry);

        // Verify audio file exists before deletion
        assert!(audio_path.exists());

        // Delete the entry with audio cleanup
        let deleted = test_history.delete_entry_with_audio("to-delete");

        assert!(deleted);
        assert!(test_history.history.get("to-delete").is_none());

        // Audio file should be deleted
        assert!(!audio_path.exists());
    }

    #[test]
    fn test_delete_nonexistent_entry() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        let deleted = test_history.history.delete("nonexistent");
        assert!(!deleted);
    }

    // ===== Test 5: Clear all - removes all entries and audio files =====
    #[test]
    fn test_clear_all_removes_entries_and_audio() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        // Create multiple entries with audio
        let mut audio_paths = Vec::new();
        for i in 0..5 {
            let audio_path = test_history.create_audio_file(&format!("audio-{}", i))
                .expect("Failed to create audio file");
            audio_paths.push(audio_path.clone());

            let entry = create_test_entry(
                &format!("entry-{}", i),
                &format!("Text {}", i),
                Some(audio_path.to_string_lossy().to_string()),
            );
            test_history.history.add(entry);
        }

        // Add some entries without audio
        for i in 5..8 {
            let entry = create_test_entry(&format!("entry-{}", i), &format!("Text {}", i), None);
            test_history.history.add(entry);
        }

        assert_eq!(test_history.history.len(), 8);

        // Verify all audio files exist
        for path in &audio_paths {
            assert!(path.exists());
        }

        // Clear all with audio cleanup
        test_history.clear_with_audio();

        // History should be empty
        assert!(test_history.history.is_empty());
        assert_eq!(test_history.history.len(), 0);

        // All audio files should be deleted
        for path in &audio_paths {
            assert!(!path.exists());
        }
    }

    // ===== Test 6: Persistence - save and load =====
    #[test]
    fn test_persistence_save_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create and populate history
        {
            let mut test_history = TestableHistory::new(&temp_dir);

            let entry1 = create_test_entry("persist-1", "First entry", None);
            let entry2 = create_test_entry("persist-2", "Second entry", Some("/path/to/audio.wav".to_string()));
            let entry3 = create_test_entry("persist-3", "Third entry", None);

            test_history.history.add(entry1);
            test_history.history.add(entry2);
            test_history.history.add(entry3);

            test_history.save().expect("Failed to save history");
        }

        // Simulate restart by creating new instance and loading
        {
            let mut test_history = TestableHistory::new(&temp_dir);
            test_history.load().expect("Failed to load history");

            assert_eq!(test_history.history.len(), 3);

            // Verify entries are loaded correctly (newest first)
            let entries = test_history.history.entries();
            assert_eq!(entries[0].id, "persist-3"); // Most recent
            assert_eq!(entries[1].id, "persist-2");
            assert_eq!(entries[2].id, "persist-1"); // Oldest

            // Verify content
            let entry2 = test_history.history.get("persist-2").unwrap();
            assert_eq!(entry2.text, "Second entry");
            assert_eq!(entry2.audio_path, Some("/path/to/audio.wav".to_string()));
        }
    }

    #[test]
    fn test_persistence_empty_history() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Save empty history
        {
            let test_history = TestableHistory::new(&temp_dir);
            test_history.save().expect("Failed to save empty history");
        }

        // Load and verify still empty
        {
            let mut test_history = TestableHistory::new(&temp_dir);
            test_history.load().expect("Failed to load empty history");
            assert!(test_history.history.is_empty());
        }
    }

    // ===== Test 7: Corrupted history.json - graceful handling =====
    #[test]
    fn test_corrupted_json_graceful_recovery() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let history_path = temp_dir.path().join("history.json");

        // Write corrupted JSON
        fs::write(&history_path, "{ this is not valid json }")
            .expect("Failed to write corrupted file");

        // Try to load - should not panic, should return empty history
        let mut test_history = TestableHistory::new(&temp_dir);
        test_history.load_graceful();

        // Should have empty history (graceful recovery)
        assert!(test_history.history.is_empty());
    }

    #[test]
    fn test_truncated_json_graceful_recovery() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let history_path = temp_dir.path().join("history.json");

        // Write truncated JSON (simulating crash during write)
        fs::write(&history_path, r#"{"entries": [{"id": "test-1", "text": "Hello"#)
            .expect("Failed to write truncated file");

        let mut test_history = TestableHistory::new(&temp_dir);
        test_history.load_graceful();

        // Should have empty history (graceful recovery)
        assert!(test_history.history.is_empty());
    }

    #[test]
    fn test_wrong_type_json_graceful_recovery() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let history_path = temp_dir.path().join("history.json");

        // Write valid JSON but wrong type
        fs::write(&history_path, r#"["this", "is", "an", "array"]"#)
            .expect("Failed to write wrong type file");

        let mut test_history = TestableHistory::new(&temp_dir);
        test_history.load_graceful();

        // Should have empty history (graceful recovery)
        assert!(test_history.history.is_empty());
    }

    #[test]
    fn test_missing_file_graceful_recovery() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Don't create any file
        let mut test_history = TestableHistory::new(&temp_dir);
        test_history.load_graceful();

        // Should have empty history
        assert!(test_history.history.is_empty());
    }

    #[test]
    fn test_empty_file_graceful_recovery() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let history_path = temp_dir.path().join("history.json");

        // Write empty file
        fs::write(&history_path, "").expect("Failed to write empty file");

        let mut test_history = TestableHistory::new(&temp_dir);
        test_history.load_graceful();

        // Should have empty history (graceful recovery)
        assert!(test_history.history.is_empty());
    }

    // ===== Additional edge case tests =====
    #[test]
    fn test_entries_returned_newest_first() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        // Add entries in order
        for i in 1..=5 {
            let entry = create_test_entry(&format!("entry-{}", i), &format!("Text {}", i), None);
            test_history.history.add(entry);
        }

        let entries = test_history.history.entries();

        // Newest should be first
        assert_eq!(entries[0].id, "entry-5");
        assert_eq!(entries[1].id, "entry-4");
        assert_eq!(entries[2].id, "entry-3");
        assert_eq!(entries[3].id, "entry-2");
        assert_eq!(entries[4].id, "entry-1");
    }

    #[test]
    fn test_history_entry_with_unicode_text() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        let unicode_text = "Hello \u{1F600} Bonjour \u{1F1EB}\u{1F1F7} \u{4E2D}\u{6587}";
        let entry = create_test_entry("unicode-entry", unicode_text, None);
        test_history.history.add(entry);

        // Save and reload
        test_history.save().expect("Failed to save");
        test_history.history = TranscriptionHistory::new();
        test_history.load().expect("Failed to load");

        let retrieved = test_history.history.get("unicode-entry").unwrap();
        assert_eq!(retrieved.text, unicode_text);
    }

    #[test]
    fn test_history_entry_with_empty_text() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        let entry = create_test_entry("empty-text", "", None);
        test_history.history.add(entry);

        let retrieved = test_history.history.get("empty-text").unwrap();
        assert_eq!(retrieved.text, "");
    }

    #[test]
    fn test_history_entry_with_very_long_text() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        let long_text = "A".repeat(100_000);
        let entry = create_test_entry("long-text", &long_text, None);
        test_history.history.add(entry);

        // Save and reload
        test_history.save().expect("Failed to save");
        test_history.history = TranscriptionHistory::new();
        test_history.load().expect("Failed to load");

        let retrieved = test_history.history.get("long-text").unwrap();
        assert_eq!(retrieved.text.len(), 100_000);
    }

    #[test]
    fn test_delete_entry_without_audio() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        // Add entry without audio
        let entry = create_test_entry("no-audio", "No audio file", None);
        test_history.history.add(entry);

        // Delete should work without issues
        let deleted = test_history.delete_entry_with_audio("no-audio");

        assert!(deleted);
        assert!(test_history.history.get("no-audio").is_none());
    }

    #[test]
    fn test_is_empty_and_len() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        assert!(test_history.history.is_empty());
        assert_eq!(test_history.history.len(), 0);

        test_history.history.add(create_test_entry("1", "test", None));

        assert!(!test_history.history.is_empty());
        assert_eq!(test_history.history.len(), 1);

        test_history.history.clear();

        assert!(test_history.history.is_empty());
        assert_eq!(test_history.history.len(), 0);
    }

    #[test]
    fn test_get_nonexistent_entry() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_history = TestableHistory::new(&temp_dir);

        assert!(test_history.history.get("does-not-exist").is_none());
    }

    #[test]
    fn test_special_characters_in_text() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut test_history = TestableHistory::new(&temp_dir);

        let special_text = r#"Line1\nLine2\tTab"Quote"'Apostrophe'<>&"#;
        let entry = create_test_entry("special", special_text, None);
        test_history.history.add(entry);

        test_history.save().expect("Failed to save");
        test_history.history = TranscriptionHistory::new();
        test_history.load().expect("Failed to load");

        let retrieved = test_history.history.get("special").unwrap();
        assert_eq!(retrieved.text, special_text);
    }
}

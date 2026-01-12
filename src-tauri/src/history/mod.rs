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

//! Streaming Transcription (Pseudo-Streaming)
//!
//! whisper-rs does not support true streaming transcription.
//! This module provides a pseudo-streaming approach using segment callbacks
//! to emit partial results during transcription.
//!
//! **Limitation**: This is NOT true real-time streaming. The entire audio
//! must still be processed, but segments are emitted as they're decoded.
//! For true streaming, consider:
//! - whisper-stream-rs (https://crates.io/crates/whisper-stream-rs)
//! - Chunked processing with overlapping windows
//!
//! **Current Implementation**: Segment callbacks for progress feedback.

use std::sync::Arc;
use parking_lot::Mutex;

/// Callback type for streaming transcription progress
pub type StreamingCallback = Box<dyn Fn(StreamingEvent) + Send + 'static>;

/// Events emitted during pseudo-streaming transcription
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamingEvent {
    /// Transcription started
    Started {
        audio_duration_ms: u64,
    },
    /// A segment has been decoded
    Segment {
        text: String,
        start_ms: i64,
        end_ms: i64,
        segment_index: i32,
        total_segments: i32,
    },
    /// Progress update (percentage)
    Progress {
        percentage: i32,
    },
    /// Transcription completed
    Completed {
        full_text: String,
        duration_ms: u64,
    },
    /// Transcription failed
    Error {
        message: String,
    },
}

/// Streaming-capable transcription state
pub struct StreamingState {
    /// Collected segments
    segments: Vec<String>,
    /// Callback for emitting events
    callback: Option<Arc<Mutex<StreamingCallback>>>,
    /// Total segments count (updated during transcription)
    total_segments: i32,
}

impl StreamingState {
    /// Create a new streaming state
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            callback: None,
            total_segments: 0,
        }
    }

    /// Create a new streaming state with callback
    pub fn with_callback(callback: StreamingCallback) -> Self {
        Self {
            segments: Vec::new(),
            callback: Some(Arc::new(Mutex::new(callback))),
            total_segments: 0,
        }
    }

    /// Emit an event through the callback
    pub fn emit(&self, event: StreamingEvent) {
        if let Some(ref cb) = self.callback {
            let cb = cb.lock();
            cb(event);
        }
    }

    /// Add a segment
    pub fn add_segment(&mut self, text: String, start_ms: i64, end_ms: i64, index: i32, total: i32) {
        self.total_segments = total;
        self.segments.push(text.clone());

        self.emit(StreamingEvent::Segment {
            text,
            start_ms,
            end_ms,
            segment_index: index,
            total_segments: total,
        });
    }

    /// Update progress
    pub fn update_progress(&self, percentage: i32) {
        self.emit(StreamingEvent::Progress { percentage });
    }

    /// Get full text from all segments
    pub fn full_text(&self) -> String {
        self.segments.join(" ")
    }

    /// Clear segments for reuse
    pub fn clear(&mut self) {
        self.segments.clear();
        self.total_segments = 0;
    }
}

impl Default for StreamingState {
    fn default() -> Self {
        Self::new()
    }
}

/// Documentation: Streaming Transcription Status
///
/// # Current Status
///
/// whisper-rs v0.14 does **not** support true streaming transcription.
/// The underlying whisper.cpp requires the full audio buffer before processing.
///
/// # What's Implemented
///
/// - **Segment callbacks**: Emit decoded segments as they're processed
/// - **Progress callbacks**: Report transcription progress percentage
/// - **Event-based API**: Structured events for UI integration
///
/// # Alternatives for True Streaming
///
/// 1. **Chunked Processing**: Process audio in overlapping windows
///    - Requires manual audio buffering and state management
///    - May introduce discontinuities at chunk boundaries
///
/// 2. **whisper-stream-rs**: Community crate for real-time transcription
///    - https://crates.io/crates/whisper-stream-rs
///    - Handles chunking and buffering automatically
///
/// 3. **Silero VAD + Chunking**: Use VAD to detect speech boundaries
///    - Process each speech segment independently
///    - Natural segmentation based on pauses
///
/// # Future Improvements
///
/// When whisper-rs adds streaming support, this module can be extended
/// to provide true real-time transcription capabilities.
pub mod docs {
    /// Streaming is not yet supported
    pub const STREAMING_STATUS: &str = "pseudo-streaming";
}

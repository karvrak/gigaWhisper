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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    // ============================================================
    // StreamingState Creation Tests
    // ============================================================

    #[test]
    fn test_new_streaming_state() {
        let state = StreamingState::new();
        assert!(state.segments.is_empty());
        assert!(state.callback.is_none());
        assert_eq!(state.total_segments, 0);
    }

    #[test]
    fn test_default_streaming_state() {
        let state = StreamingState::default();
        assert!(state.segments.is_empty());
        assert!(state.callback.is_none());
        assert_eq!(state.total_segments, 0);
    }

    #[test]
    fn test_with_callback() {
        let callback_count = Arc::new(AtomicU32::new(0));
        let counter = callback_count.clone();

        let state = StreamingState::with_callback(Box::new(move |_event| {
            counter.fetch_add(1, Ordering::SeqCst);
        }));

        assert!(state.callback.is_some());
    }

    // ============================================================
    // Segment Management Tests
    // ============================================================

    #[test]
    fn test_add_segment() {
        let mut state = StreamingState::new();

        state.add_segment("Hello".to_string(), 0, 1000, 0, 2);
        assert_eq!(state.segments.len(), 1);
        assert_eq!(state.segments[0], "Hello");
        assert_eq!(state.total_segments, 2);
    }

    #[test]
    fn test_add_multiple_segments() {
        let mut state = StreamingState::new();

        state.add_segment("Hello".to_string(), 0, 1000, 0, 3);
        state.add_segment("world".to_string(), 1000, 2000, 1, 3);
        state.add_segment("!".to_string(), 2000, 2500, 2, 3);

        assert_eq!(state.segments.len(), 3);
        assert_eq!(state.total_segments, 3);
    }

    #[test]
    fn test_add_segment_updates_total() {
        let mut state = StreamingState::new();

        state.add_segment("First".to_string(), 0, 1000, 0, 5);
        assert_eq!(state.total_segments, 5);

        // Total can be updated by subsequent calls
        state.add_segment("Second".to_string(), 1000, 2000, 1, 10);
        assert_eq!(state.total_segments, 10);
    }

    #[test]
    fn test_add_empty_segment() {
        let mut state = StreamingState::new();

        state.add_segment("".to_string(), 0, 1000, 0, 1);
        assert_eq!(state.segments.len(), 1);
        assert_eq!(state.segments[0], "");
    }

    // ============================================================
    // Full Text Assembly Tests
    // ============================================================

    #[test]
    fn test_full_text_empty() {
        let state = StreamingState::new();
        assert_eq!(state.full_text(), "");
    }

    #[test]
    fn test_full_text_single_segment() {
        let mut state = StreamingState::new();
        state.add_segment("Hello".to_string(), 0, 1000, 0, 1);
        assert_eq!(state.full_text(), "Hello");
    }

    #[test]
    fn test_full_text_multiple_segments() {
        let mut state = StreamingState::new();
        state.add_segment("Hello".to_string(), 0, 1000, 0, 2);
        state.add_segment("world".to_string(), 1000, 2000, 1, 2);
        assert_eq!(state.full_text(), "Hello world");
    }

    #[test]
    fn test_full_text_joins_with_space() {
        let mut state = StreamingState::new();
        state.add_segment("The".to_string(), 0, 500, 0, 4);
        state.add_segment("quick".to_string(), 500, 1000, 1, 4);
        state.add_segment("brown".to_string(), 1000, 1500, 2, 4);
        state.add_segment("fox".to_string(), 1500, 2000, 3, 4);
        assert_eq!(state.full_text(), "The quick brown fox");
    }

    // ============================================================
    // Clear Tests
    // ============================================================

    #[test]
    fn test_clear_empty_state() {
        let mut state = StreamingState::new();
        state.clear();
        assert!(state.segments.is_empty());
        assert_eq!(state.total_segments, 0);
    }

    #[test]
    fn test_clear_with_segments() {
        let mut state = StreamingState::new();
        state.add_segment("Hello".to_string(), 0, 1000, 0, 2);
        state.add_segment("world".to_string(), 1000, 2000, 1, 2);

        state.clear();

        assert!(state.segments.is_empty());
        assert_eq!(state.total_segments, 0);
    }

    #[test]
    fn test_clear_preserves_callback() {
        let callback_count = Arc::new(AtomicU32::new(0));
        let counter = callback_count.clone();

        let mut state = StreamingState::with_callback(Box::new(move |_event| {
            counter.fetch_add(1, Ordering::SeqCst);
        }));

        state.add_segment("Test".to_string(), 0, 1000, 0, 1);
        state.clear();

        // Callback should still be set
        assert!(state.callback.is_some());

        // And it should still work
        state.emit(StreamingEvent::Progress { percentage: 50 });
        assert!(callback_count.load(Ordering::SeqCst) > 0);
    }

    // ============================================================
    // Emit Tests
    // ============================================================

    #[test]
    fn test_emit_without_callback() {
        let state = StreamingState::new();
        // Should not panic when no callback is set
        state.emit(StreamingEvent::Progress { percentage: 50 });
    }

    #[test]
    fn test_emit_with_callback() {
        let callback_count = Arc::new(AtomicU32::new(0));
        let counter = callback_count.clone();

        let state = StreamingState::with_callback(Box::new(move |_event| {
            counter.fetch_add(1, Ordering::SeqCst);
        }));

        state.emit(StreamingEvent::Progress { percentage: 50 });
        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_emit_multiple_times() {
        let callback_count = Arc::new(AtomicU32::new(0));
        let counter = callback_count.clone();

        let state = StreamingState::with_callback(Box::new(move |_event| {
            counter.fetch_add(1, Ordering::SeqCst);
        }));

        state.emit(StreamingEvent::Progress { percentage: 25 });
        state.emit(StreamingEvent::Progress { percentage: 50 });
        state.emit(StreamingEvent::Progress { percentage: 75 });
        state.emit(StreamingEvent::Progress { percentage: 100 });

        assert_eq!(callback_count.load(Ordering::SeqCst), 4);
    }

    // ============================================================
    // Update Progress Tests
    // ============================================================

    #[test]
    fn test_update_progress_without_callback() {
        let state = StreamingState::new();
        // Should not panic
        state.update_progress(50);
    }

    #[test]
    fn test_update_progress_with_callback() {
        let last_percentage = Arc::new(AtomicU32::new(0));
        let percentage_ref = last_percentage.clone();

        let state = StreamingState::with_callback(Box::new(move |event| {
            if let StreamingEvent::Progress { percentage } = event {
                percentage_ref.store(percentage as u32, Ordering::SeqCst);
            }
        }));

        state.update_progress(75);
        assert_eq!(last_percentage.load(Ordering::SeqCst), 75);
    }

    #[test]
    fn test_update_progress_boundary_values() {
        let state = StreamingState::new();
        // Should handle all valid percentages without panic
        state.update_progress(0);
        state.update_progress(100);
        state.update_progress(-1); // Edge case
        state.update_progress(101); // Edge case
    }

    // ============================================================
    // StreamingEvent Tests
    // ============================================================

    #[test]
    fn test_streaming_event_started() {
        let event = StreamingEvent::Started { audio_duration_ms: 5000 };
        if let StreamingEvent::Started { audio_duration_ms } = event {
            assert_eq!(audio_duration_ms, 5000);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_streaming_event_segment() {
        let event = StreamingEvent::Segment {
            text: "Hello".to_string(),
            start_ms: 0,
            end_ms: 1000,
            segment_index: 0,
            total_segments: 5,
        };

        if let StreamingEvent::Segment {
            text,
            start_ms,
            end_ms,
            segment_index,
            total_segments,
        } = event
        {
            assert_eq!(text, "Hello");
            assert_eq!(start_ms, 0);
            assert_eq!(end_ms, 1000);
            assert_eq!(segment_index, 0);
            assert_eq!(total_segments, 5);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_streaming_event_progress() {
        let event = StreamingEvent::Progress { percentage: 50 };
        if let StreamingEvent::Progress { percentage } = event {
            assert_eq!(percentage, 50);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_streaming_event_completed() {
        let event = StreamingEvent::Completed {
            full_text: "Hello world".to_string(),
            duration_ms: 1500,
        };

        if let StreamingEvent::Completed {
            full_text,
            duration_ms,
        } = event
        {
            assert_eq!(full_text, "Hello world");
            assert_eq!(duration_ms, 1500);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_streaming_event_error() {
        let event = StreamingEvent::Error {
            message: "Something went wrong".to_string(),
        };

        if let StreamingEvent::Error { message } = event {
            assert_eq!(message, "Something went wrong");
        } else {
            panic!("Wrong event type");
        }
    }

    // ============================================================
    // Clone Tests for StreamingEvent
    // ============================================================

    #[test]
    fn test_streaming_event_clone() {
        let event = StreamingEvent::Segment {
            text: "Test".to_string(),
            start_ms: 0,
            end_ms: 1000,
            segment_index: 0,
            total_segments: 1,
        };

        let cloned = event.clone();
        if let StreamingEvent::Segment { text, .. } = cloned {
            assert_eq!(text, "Test");
        }
    }

    // ============================================================
    // Serialization Tests
    // ============================================================

    #[test]
    fn test_streaming_event_serialize_started() {
        let event = StreamingEvent::Started { audio_duration_ms: 5000 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("started"));
        assert!(json.contains("5000"));
    }

    #[test]
    fn test_streaming_event_serialize_segment() {
        let event = StreamingEvent::Segment {
            text: "Hello".to_string(),
            start_ms: 0,
            end_ms: 1000,
            segment_index: 0,
            total_segments: 5,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("segment"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_streaming_event_serialize_progress() {
        let event = StreamingEvent::Progress { percentage: 75 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("progress"));
        assert!(json.contains("75"));
    }

    #[test]
    fn test_streaming_event_serialize_completed() {
        let event = StreamingEvent::Completed {
            full_text: "Done".to_string(),
            duration_ms: 2000,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("completed"));
        assert!(json.contains("Done"));
    }

    #[test]
    fn test_streaming_event_serialize_error() {
        let event = StreamingEvent::Error {
            message: "Failed".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Failed"));
    }

    // ============================================================
    // Integration Tests
    // ============================================================

    #[test]
    fn test_full_transcription_flow() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_ref = events.clone();

        let mut state = StreamingState::with_callback(Box::new(move |event| {
            events_ref.lock().push(event);
        }));

        // Simulate transcription flow
        state.emit(StreamingEvent::Started { audio_duration_ms: 5000 });
        state.add_segment("Hello".to_string(), 0, 1000, 0, 3);
        state.update_progress(33);
        state.add_segment("beautiful".to_string(), 1000, 2500, 1, 3);
        state.update_progress(66);
        state.add_segment("world".to_string(), 2500, 4000, 2, 3);
        state.update_progress(100);
        state.emit(StreamingEvent::Completed {
            full_text: state.full_text(),
            duration_ms: 4000,
        });

        let captured_events = events.lock();
        assert!(captured_events.len() >= 7); // Started + 3 segments + 3 progress + completed

        // Verify full text
        assert_eq!(state.full_text(), "Hello beautiful world");
    }

    #[test]
    fn test_reuse_state_after_clear() {
        let mut state = StreamingState::new();

        // First transcription
        state.add_segment("First".to_string(), 0, 1000, 0, 1);
        assert_eq!(state.full_text(), "First");

        // Clear and reuse
        state.clear();
        assert_eq!(state.full_text(), "");

        // Second transcription
        state.add_segment("Second".to_string(), 0, 1000, 0, 1);
        assert_eq!(state.full_text(), "Second");
    }

    // ============================================================
    // Documentation Module Tests
    // ============================================================

    #[test]
    fn test_streaming_status_constant() {
        assert_eq!(docs::STREAMING_STATUS, "pseudo-streaming");
    }
}

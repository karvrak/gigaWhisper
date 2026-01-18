//! Performance Metrics
//!
//! Collect and expose performance metrics for transcription operations.
//! These metrics help users understand and optimize their configuration.

use parking_lot::RwLock;
use serde::Serialize;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Maximum number of transcription records to keep
const MAX_HISTORY: usize = 100;

/// Global metrics instance
static METRICS: once_cell::sync::Lazy<RwLock<PerformanceMetrics>> =
    once_cell::sync::Lazy::new(|| RwLock::new(PerformanceMetrics::new()));

/// Get the global metrics instance
pub fn metrics() -> &'static RwLock<PerformanceMetrics> {
    &METRICS
}

/// Performance metrics collector
pub struct PerformanceMetrics {
    /// History of transcription operations
    transcriptions: VecDeque<TranscriptionRecord>,
    /// Current session start time
    session_start: Instant,
    /// Total audio processed in this session (ms)
    total_audio_ms: u64,
    /// Total processing time in this session (ms)
    total_processing_ms: u64,
    /// Model load time (ms)
    model_load_time_ms: Option<u64>,
    /// Current model memory usage estimate (bytes)
    estimated_model_memory: Option<u64>,
}

impl PerformanceMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            transcriptions: VecDeque::with_capacity(MAX_HISTORY),
            session_start: Instant::now(),
            total_audio_ms: 0,
            total_processing_ms: 0,
            model_load_time_ms: None,
            estimated_model_memory: None,
        }
    }

    /// Record a transcription operation
    pub fn record_transcription(&mut self, record: TranscriptionRecord) {
        self.total_audio_ms += record.audio_duration_ms;
        self.total_processing_ms += record.processing_time_ms;

        if self.transcriptions.len() >= MAX_HISTORY {
            self.transcriptions.pop_front();
        }
        self.transcriptions.push_back(record);
    }

    /// Record model load time
    pub fn record_model_load(&mut self, duration: Duration, estimated_memory: u64) {
        self.model_load_time_ms = Some(duration.as_millis() as u64);
        self.estimated_model_memory = Some(estimated_memory);
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> MetricsSummary {
        let count = self.transcriptions.len();

        if count == 0 {
            return MetricsSummary::default();
        }

        let total_processing: u64 = self.transcriptions.iter().map(|r| r.processing_time_ms).sum();
        let total_audio: u64 = self.transcriptions.iter().map(|r| r.audio_duration_ms).sum();

        let avg_processing_ms = total_processing / count as u64;
        let avg_audio_ms = total_audio / count as u64;

        // Real-time factor: processing_time / audio_duration
        // < 1.0 means faster than real-time
        let avg_rtf = if total_audio > 0 {
            total_processing as f64 / total_audio as f64
        } else {
            0.0
        };

        // Find fastest and slowest
        let fastest = self.transcriptions.iter().map(|r| r.processing_time_ms).min().unwrap_or(0);
        let slowest = self.transcriptions.iter().map(|r| r.processing_time_ms).max().unwrap_or(0);

        // Calculate percentile (p95)
        let mut times: Vec<u64> = self.transcriptions.iter().map(|r| r.processing_time_ms).collect();
        times.sort();
        let p95_idx = (count as f64 * 0.95) as usize;
        let p95_ms = times.get(p95_idx.min(count - 1)).copied().unwrap_or(0);

        // VAD statistics
        let vad_records: Vec<_> = self.transcriptions.iter().filter(|r| r.vad_enabled).collect();
        let vad_savings_ms = if !vad_records.is_empty() {
            let total_original: u64 = vad_records.iter().map(|r| r.audio_duration_ms).sum();
            let total_filtered: u64 = vad_records.iter().map(|r| r.vad_filtered_ms.unwrap_or(r.audio_duration_ms)).sum();
            total_original.saturating_sub(total_filtered)
        } else {
            0
        };

        MetricsSummary {
            transcription_count: count,
            session_duration_ms: self.session_start.elapsed().as_millis() as u64,
            avg_processing_ms,
            avg_audio_ms,
            avg_real_time_factor: avg_rtf,
            fastest_ms: fastest,
            slowest_ms: slowest,
            p95_ms,
            total_audio_processed_ms: self.total_audio_ms,
            total_processing_time_ms: self.total_processing_ms,
            model_load_time_ms: self.model_load_time_ms,
            estimated_model_memory_bytes: self.estimated_model_memory,
            vad_savings_ms,
        }
    }

    /// Get recent transcription records
    pub fn get_recent(&self, count: usize) -> Vec<TranscriptionRecord> {
        self.transcriptions
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Reset metrics
    pub fn reset(&mut self) {
        self.transcriptions.clear();
        self.session_start = Instant::now();
        self.total_audio_ms = 0;
        self.total_processing_ms = 0;
        // Keep model load time as it's still valid
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Record of a single transcription operation
#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionRecord {
    /// Timestamp when transcription started (unix ms)
    pub timestamp_ms: u64,
    /// Audio duration in milliseconds
    pub audio_duration_ms: u64,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Real-time factor (processing_time / audio_duration)
    pub real_time_factor: f64,
    /// Provider used (local/groq)
    pub provider: String,
    /// Model name/size
    pub model: String,
    /// Whether GPU was used
    pub gpu_used: bool,
    /// Number of threads used
    pub threads_used: usize,
    /// Whether VAD was enabled
    pub vad_enabled: bool,
    /// Audio duration after VAD filtering (if enabled)
    pub vad_filtered_ms: Option<u64>,
    /// Number of characters in result
    pub result_chars: usize,
}

impl TranscriptionRecord {
    /// Create a new record builder
    pub fn builder() -> TranscriptionRecordBuilder {
        TranscriptionRecordBuilder::new()
    }
}

/// Builder for TranscriptionRecord
pub struct TranscriptionRecordBuilder {
    record: TranscriptionRecord,
}

impl TranscriptionRecordBuilder {
    fn new() -> Self {
        Self {
            record: TranscriptionRecord {
                timestamp_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                audio_duration_ms: 0,
                processing_time_ms: 0,
                real_time_factor: 0.0,
                provider: String::new(),
                model: String::new(),
                gpu_used: false,
                threads_used: 0,
                vad_enabled: false,
                vad_filtered_ms: None,
                result_chars: 0,
            },
        }
    }

    pub fn audio_duration_ms(mut self, ms: u64) -> Self {
        self.record.audio_duration_ms = ms;
        self
    }

    pub fn processing_time_ms(mut self, ms: u64) -> Self {
        self.record.processing_time_ms = ms;
        self.record.real_time_factor = if self.record.audio_duration_ms > 0 {
            ms as f64 / self.record.audio_duration_ms as f64
        } else {
            0.0
        };
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.record.provider = provider.into();
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.record.model = model.into();
        self
    }

    pub fn gpu_used(mut self, used: bool) -> Self {
        self.record.gpu_used = used;
        self
    }

    pub fn threads_used(mut self, threads: usize) -> Self {
        self.record.threads_used = threads;
        self
    }

    pub fn vad_enabled(mut self, enabled: bool) -> Self {
        self.record.vad_enabled = enabled;
        self
    }

    pub fn vad_filtered_ms(mut self, ms: u64) -> Self {
        self.record.vad_filtered_ms = Some(ms);
        self
    }

    pub fn result_chars(mut self, chars: usize) -> Self {
        self.record.result_chars = chars;
        self
    }

    pub fn build(self) -> TranscriptionRecord {
        self.record
    }
}

/// Summary of performance metrics
#[derive(Debug, Clone, Serialize, Default)]
pub struct MetricsSummary {
    /// Number of transcriptions recorded
    pub transcription_count: usize,
    /// Session duration in ms
    pub session_duration_ms: u64,
    /// Average processing time per transcription (ms)
    pub avg_processing_ms: u64,
    /// Average audio duration per transcription (ms)
    pub avg_audio_ms: u64,
    /// Average real-time factor (< 1.0 = faster than real-time)
    pub avg_real_time_factor: f64,
    /// Fastest transcription time (ms)
    pub fastest_ms: u64,
    /// Slowest transcription time (ms)
    pub slowest_ms: u64,
    /// 95th percentile processing time (ms)
    pub p95_ms: u64,
    /// Total audio processed in session (ms)
    pub total_audio_processed_ms: u64,
    /// Total processing time in session (ms)
    pub total_processing_time_ms: u64,
    /// Model load time (ms)
    pub model_load_time_ms: Option<u64>,
    /// Estimated model memory usage (bytes)
    pub estimated_model_memory_bytes: Option<u64>,
    /// Total audio saved by VAD filtering (ms)
    pub vad_savings_ms: u64,
}

impl MetricsSummary {
    /// Format memory size for display
    pub fn format_memory(&self) -> String {
        match self.estimated_model_memory_bytes {
            Some(bytes) if bytes >= 1_000_000_000 => {
                format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
            }
            Some(bytes) if bytes >= 1_000_000 => {
                format!("{:.1} MB", bytes as f64 / 1_000_000.0)
            }
            Some(bytes) => format!("{} KB", bytes / 1000),
            None => "Unknown".to_string(),
        }
    }

    /// Get performance rating based on RTF
    pub fn performance_rating(&self) -> &'static str {
        match self.avg_real_time_factor {
            rtf if rtf < 0.1 => "Excellent",
            rtf if rtf < 0.3 => "Very Good",
            rtf if rtf < 0.5 => "Good",
            rtf if rtf < 1.0 => "Fair",
            _ => "Needs Optimization",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let mut metrics = PerformanceMetrics::new();

        let record = TranscriptionRecord::builder()
            .audio_duration_ms(5000)
            .processing_time_ms(1000)
            .provider("local")
            .model("small")
            .threads_used(4)
            .build();

        metrics.record_transcription(record);

        let summary = metrics.get_summary();
        assert_eq!(summary.transcription_count, 1);
        assert_eq!(summary.avg_processing_ms, 1000);
        assert!((summary.avg_real_time_factor - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_rtf_calculation() {
        let record = TranscriptionRecord::builder()
            .audio_duration_ms(10000)
            .processing_time_ms(2000)
            .build();

        assert!((record.real_time_factor - 0.2).abs() < 0.01);
    }
}

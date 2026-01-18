//! Voice Activity Detection (VAD)
//!
//! Sophisticated voice activity detection using WebRTC VAD.
//! This helps reduce processing time by filtering out silent segments
//! before sending audio to whisper.cpp.

use webrtc_vad::{Vad, SampleRate, VadMode};

/// VAD aggressiveness level (maps to WebRTC VadMode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VadAggressiveness {
    /// Quality mode - least aggressive, fewest false negatives
    Quality = 0,
    /// Low bitrate mode
    LowBitrate = 1,
    /// Aggressive mode - good balance
    #[default]
    Aggressive = 2,
    /// Very aggressive - most aggressive filtering
    VeryAggressive = 3,
}

impl VadAggressiveness {
    /// Convert to WebRTC VadMode
    fn to_vad_mode(self) -> VadMode {
        match self {
            Self::Quality => VadMode::Quality,
            Self::LowBitrate => VadMode::LowBitrate,
            Self::Aggressive => VadMode::Aggressive,
            Self::VeryAggressive => VadMode::VeryAggressive,
        }
    }
}

/// VAD configuration
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// VAD aggressiveness mode (0-3, higher = more aggressive filtering)
    pub mode: VadAggressiveness,
    /// Minimum speech duration in milliseconds to keep
    pub min_speech_duration_ms: u32,
    /// Padding to add around detected speech segments (ms)
    pub padding_ms: u32,
    /// Frame size in samples (must be 10, 20, or 30ms worth at the sample rate)
    pub frame_duration_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            // Mode 2 (Aggressive) is a good balance
            // Mode 0 = Quality, Mode 3 = Very Aggressive
            mode: VadAggressiveness::Aggressive,
            min_speech_duration_ms: 100, // Ignore speech shorter than 100ms
            padding_ms: 300,             // Add 300ms padding around speech
            frame_duration_ms: 30,       // 30ms frames (max supported)
        }
    }
}

/// VAD processing result
#[derive(Debug, Clone)]
pub struct VadResult {
    /// Filtered audio containing only speech segments
    pub audio: Vec<f32>,
    /// Total duration of original audio in ms
    pub original_duration_ms: u64,
    /// Total duration of speech detected in ms
    pub speech_duration_ms: u64,
    /// Number of speech segments detected
    pub speech_segments: usize,
    /// Percentage of audio that was speech
    pub speech_percentage: f32,
}

/// Voice Activity Detector using WebRTC VAD
pub struct VoiceActivityDetector {
    config: VadConfig,
}

impl VoiceActivityDetector {
    /// Create a new VAD with default configuration
    pub fn new() -> Self {
        Self {
            config: VadConfig::default(),
        }
    }

    /// Create a new VAD with custom configuration
    pub fn with_config(config: VadConfig) -> Self {
        Self { config }
    }

    /// Process audio and return only the speech segments
    ///
    /// The input audio must be:
    /// - Mono (single channel)
    /// - 16kHz sample rate (required by WebRTC VAD and Whisper)
    /// - f32 format (will be converted internally)
    pub fn filter_speech(&self, audio: &[f32], sample_rate: u32) -> Result<VadResult, VadError> {
        // WebRTC VAD only supports 8kHz, 16kHz, 32kHz, 48kHz
        let vad_sample_rate = match sample_rate {
            8000 => SampleRate::Rate8kHz,
            16000 => SampleRate::Rate16kHz,
            32000 => SampleRate::Rate32kHz,
            48000 => SampleRate::Rate48kHz,
            _ => return Err(VadError::UnsupportedSampleRate(sample_rate)),
        };

        // Create VAD instance
        let mut vad = Vad::new_with_rate_and_mode(vad_sample_rate, self.config.mode.to_vad_mode());

        // Calculate frame size in samples
        let frame_samples = (sample_rate * self.config.frame_duration_ms / 1000) as usize;

        // Convert f32 to i16 for WebRTC VAD
        let audio_i16: Vec<i16> = audio
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        // Process frames and detect speech
        let mut speech_frames: Vec<bool> = Vec::new();
        for chunk in audio_i16.chunks(frame_samples) {
            if chunk.len() == frame_samples {
                let is_speech = vad.is_voice_segment(chunk).unwrap_or(false);
                speech_frames.push(is_speech);
            } else {
                // For the last partial frame, assume it's speech to avoid cutting off
                speech_frames.push(true);
            }
        }

        // Apply minimum speech duration filter
        let min_frames = (self.config.min_speech_duration_ms / self.config.frame_duration_ms) as usize;
        let speech_frames = filter_short_segments(&speech_frames, min_frames);

        // Apply padding around speech segments
        let padding_frames = (self.config.padding_ms / self.config.frame_duration_ms) as usize;
        let speech_frames = apply_padding(&speech_frames, padding_frames);

        // Extract speech segments
        let mut result_audio: Vec<f32> = Vec::new();
        let mut speech_segments = 0;
        let mut in_speech = false;

        for (i, &is_speech) in speech_frames.iter().enumerate() {
            let start_sample = i * frame_samples;
            let end_sample = ((i + 1) * frame_samples).min(audio.len());

            if is_speech {
                if !in_speech {
                    speech_segments += 1;
                    in_speech = true;
                }
                result_audio.extend_from_slice(&audio[start_sample..end_sample]);
            } else {
                in_speech = false;
            }
        }

        // Calculate statistics
        let original_duration_ms = (audio.len() as u64 * 1000) / sample_rate as u64;
        let speech_duration_ms = (result_audio.len() as u64 * 1000) / sample_rate as u64;
        let speech_percentage = if original_duration_ms > 0 {
            (speech_duration_ms as f32 / original_duration_ms as f32) * 100.0
        } else {
            0.0
        };

        tracing::debug!(
            "VAD: {:.1}% speech detected ({} segments, {}ms -> {}ms)",
            speech_percentage,
            speech_segments,
            original_duration_ms,
            speech_duration_ms
        );

        Ok(VadResult {
            audio: result_audio,
            original_duration_ms,
            speech_duration_ms,
            speech_segments,
            speech_percentage,
        })
    }

    /// Check if audio contains any speech (quick check without filtering)
    pub fn contains_speech(&self, audio: &[f32], sample_rate: u32) -> Result<bool, VadError> {
        let vad_sample_rate = match sample_rate {
            8000 => SampleRate::Rate8kHz,
            16000 => SampleRate::Rate16kHz,
            32000 => SampleRate::Rate32kHz,
            48000 => SampleRate::Rate48kHz,
            _ => return Err(VadError::UnsupportedSampleRate(sample_rate)),
        };

        let mut vad = Vad::new_with_rate_and_mode(vad_sample_rate, self.config.mode.to_vad_mode());

        let frame_samples = (sample_rate * self.config.frame_duration_ms / 1000) as usize;

        let audio_i16: Vec<i16> = audio
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        // Check first few frames only for quick detection
        let frames_to_check = 10.min(audio_i16.len() / frame_samples);
        let mut speech_count = 0;

        for i in 0..frames_to_check {
            let start = i * frame_samples;
            let end = start + frame_samples;
            if end <= audio_i16.len() {
                if vad.is_voice_segment(&audio_i16[start..end]).unwrap_or(false) {
                    speech_count += 1;
                }
            }
        }

        // Consider it has speech if at least 20% of checked frames have speech
        Ok(speech_count > frames_to_check / 5)
    }
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// VAD errors
#[derive(Debug, thiserror::Error)]
pub enum VadError {
    #[error("Unsupported sample rate: {0}Hz. WebRTC VAD supports 8kHz, 16kHz, 32kHz, 48kHz")]
    UnsupportedSampleRate(u32),

    #[error("VAD processing failed: {0}")]
    ProcessingError(String),
}

/// Filter out speech segments shorter than min_frames
fn filter_short_segments(frames: &[bool], min_frames: usize) -> Vec<bool> {
    if min_frames <= 1 {
        return frames.to_vec();
    }

    let mut result = frames.to_vec();
    let mut segment_start: Option<usize> = None;

    for i in 0..=frames.len() {
        let is_speech = i < frames.len() && frames[i];

        match (segment_start, is_speech) {
            (None, true) => {
                segment_start = Some(i);
            }
            (Some(start), false) => {
                let segment_len = i - start;
                if segment_len < min_frames {
                    // Mark short segment as non-speech
                    for j in start..i {
                        result[j] = false;
                    }
                }
                segment_start = None;
            }
            _ => {}
        }
    }

    result
}

/// Apply padding around speech segments
fn apply_padding(frames: &[bool], padding: usize) -> Vec<bool> {
    if padding == 0 {
        return frames.to_vec();
    }

    let mut result = frames.to_vec();

    // Forward pass: extend speech forward
    let mut countdown = 0;
    for i in 0..frames.len() {
        if frames[i] {
            countdown = padding;
        } else if countdown > 0 {
            result[i] = true;
            countdown -= 1;
        }
    }

    // Backward pass: extend speech backward
    countdown = 0;
    for i in (0..frames.len()).rev() {
        if frames[i] {
            countdown = padding;
        } else if countdown > 0 {
            result[i] = true;
            countdown -= 1;
        }
    }

    result
}

/// Simple RMS-based voice detection (fallback/complement to WebRTC VAD)
pub fn calculate_rms(audio: &[f32]) -> f32 {
    if audio.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = audio.iter().map(|&s| s * s).sum();
    (sum_squares / audio.len() as f32).sqrt()
}

/// Check if audio level is above a threshold (simple VAD)
pub fn is_above_threshold(audio: &[f32], threshold_db: f32) -> bool {
    let rms = calculate_rms(audio);
    let db = 20.0 * rms.log10();
    db > threshold_db
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_short_segments() {
        let frames = vec![false, true, false, true, true, true, false, true, false];
        let result = filter_short_segments(&frames, 2);
        // Single frame of speech should be filtered out
        assert!(!result[1]);
        // Three consecutive frames should remain
        assert!(result[3] && result[4] && result[5]);
    }

    #[test]
    fn test_apply_padding() {
        let frames = vec![false, false, true, true, false, false, false];
        let result = apply_padding(&frames, 1);
        // Should add 1 frame before and after speech
        assert_eq!(result, vec![false, true, true, true, true, false, false]);
    }

    #[test]
    fn test_rms_calculation() {
        let silence = vec![0.0; 100];
        assert_eq!(calculate_rms(&silence), 0.0);

        let signal = vec![0.5; 100];
        assert!((calculate_rms(&signal) - 0.5).abs() < 0.001);
    }
}

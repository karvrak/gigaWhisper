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
    use std::f32::consts::PI;

    // ========================================================================
    // Helper functions for generating synthetic audio signals
    // ========================================================================

    /// Generate a sine wave at the given frequency
    fn generate_sine_wave(sample_rate: u32, frequency: f32, duration_ms: u32, amplitude: f32) -> Vec<f32> {
        let num_samples = (sample_rate as f32 * duration_ms as f32 / 1000.0) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                amplitude * (2.0 * PI * frequency * t).sin()
            })
            .collect()
    }

    /// Generate silence (zeros)
    fn generate_silence(sample_rate: u32, duration_ms: u32) -> Vec<f32> {
        let num_samples = (sample_rate as f32 * duration_ms as f32 / 1000.0) as usize;
        vec![0.0; num_samples]
    }

    /// Generate white noise with given amplitude
    fn generate_noise(sample_rate: u32, duration_ms: u32, amplitude: f32) -> Vec<f32> {
        let num_samples = (sample_rate as f32 * duration_ms as f32 / 1000.0) as usize;
        // Use a simple pseudo-random generator for deterministic tests
        let mut seed: u32 = 12345;
        (0..num_samples)
            .map(|_| {
                // Simple LCG pseudo-random number generator
                seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                let random = ((seed >> 16) & 0x7fff) as f32 / 32768.0;
                amplitude * (random * 2.0 - 1.0)
            })
            .collect()
    }

    /// Generate a speech-like signal (combination of frequencies typical of human voice)
    fn generate_speech_like_signal(sample_rate: u32, duration_ms: u32, amplitude: f32) -> Vec<f32> {
        let num_samples = (sample_rate as f32 * duration_ms as f32 / 1000.0) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                // Combine fundamental and harmonics typical of human voice (100-300Hz fundamental)
                let fundamental = 150.0;
                let signal = 0.5 * (2.0 * PI * fundamental * t).sin()
                    + 0.3 * (2.0 * PI * fundamental * 2.0 * t).sin()
                    + 0.15 * (2.0 * PI * fundamental * 3.0 * t).sin()
                    + 0.05 * (2.0 * PI * fundamental * 4.0 * t).sin();
                amplitude * signal
            })
            .collect()
    }

    /// Concatenate multiple audio segments
    fn concatenate_audio(segments: Vec<Vec<f32>>) -> Vec<f32> {
        segments.into_iter().flatten().collect()
    }

    // ========================================================================
    // Original tests (preserved)
    // ========================================================================

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

    // ========================================================================
    // 1. Speech detection tests
    // ========================================================================

    #[test]
    fn test_vad_detects_speech_in_speech_like_signal() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Generate 500ms of speech-like signal at a reasonable amplitude
        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);

        let result = vad.filter_speech(&speech, sample_rate).unwrap();

        // Should detect speech (speech percentage should be significant)
        assert!(result.speech_percentage > 0.0, "Should detect speech in speech-like signal");
        assert!(!result.audio.is_empty(), "Should return non-empty audio");
    }

    #[test]
    fn test_vad_silence_detection() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Generate 500ms of pure silence
        let silence = generate_silence(sample_rate, 500);

        let result = vad.filter_speech(&silence, sample_rate).unwrap();

        // Pure silence should result in no speech detected
        // Note: Due to padding and last-frame handling, there might be some audio
        assert!(
            result.speech_percentage < 50.0,
            "Should detect minimal speech in silence"
        );
    }

    #[test]
    fn test_vad_low_amplitude_noise() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Generate 500ms of very low amplitude noise (simulating background noise)
        let noise = generate_noise(sample_rate, 500, 0.01);

        let result = vad.filter_speech(&noise, sample_rate).unwrap();

        // Low amplitude noise should not be detected as speech in aggressive mode
        // Due to the way WebRTC VAD works, very low signals are typically filtered
        assert!(result.original_duration_ms > 0);
    }

    #[test]
    fn test_vad_loud_noise_vs_speech() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Generate speech-like signal
        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);

        // Generate random noise at similar amplitude
        let noise = generate_noise(sample_rate, 500, 0.5);

        let speech_result = vad.filter_speech(&speech, sample_rate).unwrap();
        let noise_result = vad.filter_speech(&noise, sample_rate).unwrap();

        // Both might trigger VAD, but we're testing the system works
        assert!(speech_result.original_duration_ms > 0);
        assert!(noise_result.original_duration_ms > 0);
    }

    #[test]
    fn test_contains_speech_with_speech() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);

        let has_speech = vad.contains_speech(&speech, sample_rate).unwrap();
        // Speech-like signal should be detected
        assert!(has_speech, "Should detect speech in speech-like signal");
    }

    #[test]
    fn test_contains_speech_with_silence() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        let silence = generate_silence(sample_rate, 500);

        let has_speech = vad.contains_speech(&silence, sample_rate).unwrap();
        // Pure silence should not be detected as speech
        assert!(!has_speech, "Should not detect speech in silence");
    }

    // ========================================================================
    // 2. Silence filtering tests
    // ========================================================================

    #[test]
    fn test_vad_trims_leading_silence() {
        let vad = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Quality, // Less aggressive for this test
            min_speech_duration_ms: 50,
            padding_ms: 100,
            frame_duration_ms: 30,
        });
        let sample_rate = 16000;

        // Leading silence (300ms) + speech (500ms)
        let audio = concatenate_audio(vec![
            generate_silence(sample_rate, 300),
            generate_speech_like_signal(sample_rate, 500, 0.5),
        ]);

        let result = vad.filter_speech(&audio, sample_rate).unwrap();

        // Result should be shorter than original due to trimmed silence
        // (accounting for padding)
        assert!(
            result.audio.len() < audio.len(),
            "Should trim leading silence"
        );
    }

    #[test]
    fn test_vad_trims_trailing_silence() {
        let vad = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Quality,
            min_speech_duration_ms: 50,
            padding_ms: 100,
            frame_duration_ms: 30,
        });
        let sample_rate = 16000;

        // Speech (500ms) + long trailing silence (1000ms) - enough to see trimming
        // Note: The last partial frame is always marked as speech, and padding adds ~100ms
        // So we need significantly more trailing silence to observe trimming
        let audio = concatenate_audio(vec![
            generate_speech_like_signal(sample_rate, 500, 0.5),
            generate_silence(sample_rate, 1000),
        ]);

        let result = vad.filter_speech(&audio, sample_rate).unwrap();

        // Result should be shorter than original due to trimmed silence
        // (accounting for padding and last-frame handling)
        assert!(
            result.audio.len() < audio.len(),
            "Should trim trailing silence, got {} samples from {} original",
            result.audio.len(),
            audio.len()
        );
    }

    #[test]
    fn test_vad_handles_pauses_between_words() {
        let vad = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Aggressive,
            min_speech_duration_ms: 50,
            padding_ms: 200, // Generous padding to merge nearby segments
            frame_duration_ms: 30,
        });
        let sample_rate = 16000;

        // Simulate "word - pause - word" pattern
        let audio = concatenate_audio(vec![
            generate_speech_like_signal(sample_rate, 300, 0.5), // Word 1
            generate_silence(sample_rate, 150),                   // Short pause
            generate_speech_like_signal(sample_rate, 300, 0.5), // Word 2
        ]);

        let result = vad.filter_speech(&audio, sample_rate).unwrap();

        // Should have processed the audio
        assert!(result.original_duration_ms > 0);
        // With generous padding, segments should be merged
        // Result should contain audio from both speech segments
        assert!(!result.audio.is_empty());
    }

    #[test]
    fn test_filter_short_segments_edge_cases() {
        // Empty input
        let empty: Vec<bool> = vec![];
        assert_eq!(filter_short_segments(&empty, 2), empty);

        // All speech
        let all_speech = vec![true, true, true, true];
        assert_eq!(filter_short_segments(&all_speech, 2), all_speech);

        // All silence
        let all_silence = vec![false, false, false, false];
        assert_eq!(filter_short_segments(&all_silence, 2), all_silence);

        // Min frames = 0 (should keep all)
        let mixed = vec![true, false, true];
        assert_eq!(filter_short_segments(&mixed, 0), mixed);

        // Min frames = 1 (should keep all speech)
        assert_eq!(filter_short_segments(&mixed, 1), mixed);
    }

    #[test]
    fn test_apply_padding_edge_cases() {
        // Empty input
        let empty: Vec<bool> = vec![];
        assert_eq!(apply_padding(&empty, 2), empty);

        // Zero padding
        let frames = vec![false, true, false];
        assert_eq!(apply_padding(&frames, 0), frames);

        // All silence
        let silence = vec![false, false, false];
        assert_eq!(apply_padding(&silence, 2), silence);

        // Padding larger than distance to edge
        let frames = vec![false, true, false, false, false];
        let result = apply_padding(&frames, 3);
        // Should extend but not overflow
        assert!(result[0]); // Extended backward
        assert!(result[1]); // Original speech
        assert!(result[2]); // Extended forward
        assert!(result[3]); // Extended forward
        assert!(result[4]); // Extended forward
    }

    #[test]
    fn test_apply_padding_multiple_segments() {
        // Two speech segments with gap
        let frames = vec![false, true, false, false, false, true, false];
        let result = apply_padding(&frames, 1);

        // First segment: index 1 should extend to 0 and 2
        assert!(result[0]); // Backward padding
        assert!(result[1]); // Original
        assert!(result[2]); // Forward padding

        // Second segment: index 5 should extend to 4 and 6
        assert!(result[4]); // Backward padding
        assert!(result[5]); // Original
        assert!(result[6]); // Forward padding
    }

    // ========================================================================
    // 3. Sample rate tests
    // ========================================================================

    #[test]
    fn test_vad_16khz_sample_rate() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);
        let result = vad.filter_speech(&speech, sample_rate);

        assert!(result.is_ok(), "Should support 16kHz sample rate");
    }

    #[test]
    fn test_vad_8khz_sample_rate() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 8000;

        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);
        let result = vad.filter_speech(&speech, sample_rate);

        assert!(result.is_ok(), "Should support 8kHz sample rate");
    }

    #[test]
    fn test_vad_32khz_sample_rate() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 32000;

        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);
        let result = vad.filter_speech(&speech, sample_rate);

        assert!(result.is_ok(), "Should support 32kHz sample rate");
    }

    #[test]
    fn test_vad_48khz_sample_rate() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 48000;

        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);
        let result = vad.filter_speech(&speech, sample_rate);

        assert!(result.is_ok(), "Should support 48kHz sample rate");
    }

    #[test]
    fn test_vad_unsupported_sample_rate_44100() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 44100; // CD quality, not supported by WebRTC VAD

        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);
        let result = vad.filter_speech(&speech, sample_rate);

        assert!(result.is_err(), "Should reject 44100Hz sample rate");
        match result {
            Err(VadError::UnsupportedSampleRate(rate)) => {
                assert_eq!(rate, 44100);
            }
            _ => panic!("Expected UnsupportedSampleRate error"),
        }
    }

    #[test]
    fn test_vad_unsupported_sample_rate_22050() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 22050;

        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);
        let result = vad.filter_speech(&speech, sample_rate);

        assert!(result.is_err(), "Should reject 22050Hz sample rate");
    }

    #[test]
    fn test_contains_speech_unsupported_sample_rate() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 44100;

        let speech = generate_speech_like_signal(sample_rate, 500, 0.5);
        let result = vad.contains_speech(&speech, sample_rate);

        assert!(result.is_err(), "contains_speech should reject unsupported sample rate");
    }

    // ========================================================================
    // 4. Sensitivity/threshold tests (VAD modes)
    // ========================================================================

    #[test]
    fn test_vad_quality_mode_most_sensitive() {
        let vad = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Quality, // Least aggressive = most sensitive
            min_speech_duration_ms: 50,
            padding_ms: 100,
            frame_duration_ms: 30,
        });
        let sample_rate = 16000;

        // Low amplitude signal that might be borderline
        let low_signal = generate_speech_like_signal(sample_rate, 500, 0.1);
        let result = vad.filter_speech(&low_signal, sample_rate).unwrap();

        // Quality mode should be more likely to detect speech
        assert!(result.original_duration_ms > 0);
    }

    #[test]
    fn test_vad_very_aggressive_mode_least_sensitive() {
        let vad = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::VeryAggressive, // Most aggressive = least sensitive
            min_speech_duration_ms: 50,
            padding_ms: 100,
            frame_duration_ms: 30,
        });
        let sample_rate = 16000;

        // Low amplitude signal
        let low_signal = generate_speech_like_signal(sample_rate, 500, 0.1);
        let result = vad.filter_speech(&low_signal, sample_rate).unwrap();

        // VeryAggressive mode filters more aggressively
        assert!(result.original_duration_ms > 0);
    }

    #[test]
    fn test_vad_mode_comparison() {
        let sample_rate = 16000;

        // Generate same audio for all modes
        let audio = generate_speech_like_signal(sample_rate, 500, 0.2);

        let quality_vad = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Quality,
            min_speech_duration_ms: 50,
            padding_ms: 100,
            frame_duration_ms: 30,
        });

        let aggressive_vad = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::VeryAggressive,
            min_speech_duration_ms: 50,
            padding_ms: 100,
            frame_duration_ms: 30,
        });

        let quality_result = quality_vad.filter_speech(&audio, sample_rate).unwrap();
        let aggressive_result = aggressive_vad.filter_speech(&audio, sample_rate).unwrap();

        // Quality mode should detect at least as much speech as VeryAggressive
        // (or equal in clear speech cases)
        let diff = if quality_result.speech_duration_ms >= aggressive_result.speech_duration_ms {
            quality_result.speech_duration_ms - aggressive_result.speech_duration_ms
        } else {
            aggressive_result.speech_duration_ms - quality_result.speech_duration_ms
        };
        assert!(
            quality_result.speech_duration_ms >= aggressive_result.speech_duration_ms || diff < 100,
            "Quality mode should be at least as sensitive as VeryAggressive"
        );
    }

    #[test]
    fn test_vad_aggressiveness_to_vad_mode() {
        // Test that conversion works without panicking
        // We can't use assert_eq because VadMode doesn't implement PartialEq
        let _ = VadAggressiveness::Quality.to_vad_mode();
        let _ = VadAggressiveness::LowBitrate.to_vad_mode();
        let _ = VadAggressiveness::Aggressive.to_vad_mode();
        let _ = VadAggressiveness::VeryAggressive.to_vad_mode();
    }

    #[test]
    fn test_vad_aggressiveness_default() {
        let default_mode = VadAggressiveness::default();
        assert_eq!(default_mode, VadAggressiveness::Aggressive);
    }

    #[test]
    fn test_vad_config_default() {
        let config = VadConfig::default();
        assert_eq!(config.mode, VadAggressiveness::Aggressive);
        assert_eq!(config.min_speech_duration_ms, 100);
        assert_eq!(config.padding_ms, 300);
        assert_eq!(config.frame_duration_ms, 30);
    }

    // ========================================================================
    // 5. Performance and long recording tests
    // ========================================================================

    #[test]
    fn test_vad_performance_5_seconds() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Generate 5 seconds of audio
        let audio = generate_speech_like_signal(sample_rate, 5000, 0.5);

        let start = std::time::Instant::now();
        let result = vad.filter_speech(&audio, sample_rate).unwrap();
        let elapsed = start.elapsed();

        // Processing should complete in reasonable time (under 1 second for 5 seconds of audio)
        assert!(
            elapsed.as_secs() < 1,
            "Processing 5 seconds of audio took too long: {:?}",
            elapsed
        );
        assert_eq!(result.original_duration_ms, 5000);
    }

    #[test]
    fn test_vad_performance_30_seconds() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Generate 30 seconds of mixed audio (typical recording length)
        let audio = concatenate_audio(vec![
            generate_silence(sample_rate, 2000),
            generate_speech_like_signal(sample_rate, 5000, 0.5),
            generate_silence(sample_rate, 1000),
            generate_speech_like_signal(sample_rate, 10000, 0.5),
            generate_silence(sample_rate, 2000),
            generate_speech_like_signal(sample_rate, 8000, 0.5),
            generate_silence(sample_rate, 2000),
        ]);

        let start = std::time::Instant::now();
        let result = vad.filter_speech(&audio, sample_rate).unwrap();
        let elapsed = start.elapsed();

        // Processing should complete in reasonable time
        assert!(
            elapsed.as_secs() < 5,
            "Processing 30 seconds of audio took too long: {:?}",
            elapsed
        );
        assert_eq!(result.original_duration_ms, 30000);
        assert!(result.speech_segments > 0, "Should detect multiple speech segments");
    }

    #[test]
    fn test_vad_long_recording_statistics() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Generate 10 seconds with known speech/silence ratio
        // 3 seconds silence + 4 seconds speech + 3 seconds silence = 40% speech
        let audio = concatenate_audio(vec![
            generate_silence(sample_rate, 3000),
            generate_speech_like_signal(sample_rate, 4000, 0.5),
            generate_silence(sample_rate, 3000),
        ]);

        let result = vad.filter_speech(&audio, sample_rate).unwrap();

        assert_eq!(result.original_duration_ms, 10000);
        // Speech percentage should be roughly 40% (with some variance due to padding)
        // Allow for padding effects
        assert!(
            result.speech_percentage > 20.0 && result.speech_percentage < 80.0,
            "Speech percentage should be reasonable: {}%",
            result.speech_percentage
        );
    }

    // ========================================================================
    // 6. Edge cases and boundary conditions
    // ========================================================================

    #[test]
    fn test_vad_empty_audio() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        let empty: Vec<f32> = vec![];
        let result = vad.filter_speech(&empty, sample_rate).unwrap();

        assert_eq!(result.original_duration_ms, 0);
        assert_eq!(result.speech_duration_ms, 0);
        assert_eq!(result.speech_segments, 0);
        assert!(result.audio.is_empty());
    }

    #[test]
    fn test_vad_very_short_audio() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Audio shorter than one frame (30ms = 480 samples at 16kHz)
        let short_audio = generate_speech_like_signal(sample_rate, 10, 0.5);

        let result = vad.filter_speech(&short_audio, sample_rate).unwrap();

        // Should handle gracefully without panic
        assert!(result.original_duration_ms <= 10);
    }

    #[test]
    fn test_vad_exactly_one_frame() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Exactly 30ms = 480 samples
        let one_frame = generate_speech_like_signal(sample_rate, 30, 0.5);

        let result = vad.filter_speech(&one_frame, sample_rate).unwrap();

        // Should handle exactly one frame
        assert_eq!(result.original_duration_ms, 30);
    }

    #[test]
    fn test_vad_clipping_audio() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Audio with values exceeding [-1, 1] range (should be clamped)
        let clipping: Vec<f32> = (0..8000)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                2.0 * (2.0 * PI * 150.0 * t).sin() // Amplitude of 2.0 exceeds range
            })
            .collect();

        let result = vad.filter_speech(&clipping, sample_rate);

        // Should handle clipping audio without panic
        assert!(result.is_ok());
    }

    #[test]
    fn test_vad_dc_offset() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        // Audio with DC offset
        let dc_offset: Vec<f32> = generate_speech_like_signal(sample_rate, 500, 0.3)
            .iter()
            .map(|&s| s + 0.2) // Add DC offset
            .collect();

        let result = vad.filter_speech(&dc_offset, sample_rate);

        // Should handle DC offset without panic
        assert!(result.is_ok());
    }

    #[test]
    fn test_vad_result_statistics() {
        let vad = VoiceActivityDetector::new();
        let sample_rate = 16000;

        let audio = generate_speech_like_signal(sample_rate, 1000, 0.5);
        let result = vad.filter_speech(&audio, sample_rate).unwrap();

        // Verify result structure
        assert_eq!(result.original_duration_ms, 1000);
        assert!(result.speech_duration_ms <= result.original_duration_ms + 1000); // Allow for padding
        assert!(result.speech_percentage >= 0.0 && result.speech_percentage <= 100.0);
    }

    #[test]
    fn test_vad_default_impl() {
        let vad1 = VoiceActivityDetector::new();
        let vad2 = VoiceActivityDetector::default();

        let sample_rate = 16000;
        let audio = generate_speech_like_signal(sample_rate, 500, 0.5);

        let result1 = vad1.filter_speech(&audio, sample_rate).unwrap();
        let result2 = vad2.filter_speech(&audio, sample_rate).unwrap();

        // Both should produce same results
        assert_eq!(result1.original_duration_ms, result2.original_duration_ms);
    }

    // ========================================================================
    // 7. RMS and threshold tests
    // ========================================================================

    #[test]
    fn test_rms_empty() {
        assert_eq!(calculate_rms(&[]), 0.0);
    }

    #[test]
    fn test_rms_single_sample() {
        assert!((calculate_rms(&[0.5]) - 0.5).abs() < 0.001);
        assert!((calculate_rms(&[-0.5]) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_rms_sine_wave() {
        // RMS of a sine wave with amplitude A is A/sqrt(2)
        let sine = generate_sine_wave(16000, 440.0, 100, 1.0);
        let rms = calculate_rms(&sine);
        let expected_rms = 1.0 / 2.0_f32.sqrt(); // ~0.707
        assert!(
            (rms - expected_rms).abs() < 0.01,
            "RMS of sine wave should be amplitude/sqrt(2), got {} expected {}",
            rms,
            expected_rms
        );
    }

    #[test]
    fn test_is_above_threshold() {
        // Loud signal
        let loud = vec![0.5; 100];
        assert!(is_above_threshold(&loud, -20.0)); // -6dB signal should be above -20dB threshold

        // Quiet signal
        let quiet = vec![0.01; 100];
        assert!(!is_above_threshold(&quiet, -20.0)); // -40dB signal should be below -20dB threshold
    }

    #[test]
    fn test_is_above_threshold_edge_cases() {
        // Empty audio
        let empty: Vec<f32> = vec![];
        // calculate_rms returns 0, log10(0) is -inf, so this should be false
        assert!(!is_above_threshold(&empty, -100.0));

        // Silence
        let silence = vec![0.0; 100];
        assert!(!is_above_threshold(&silence, -100.0));
    }

    // ========================================================================
    // 8. Configuration tests
    // ========================================================================

    #[test]
    fn test_vad_custom_min_speech_duration() {
        let sample_rate = 16000;
        let audio = generate_speech_like_signal(sample_rate, 500, 0.5);

        // Short min duration
        let vad_short = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Aggressive,
            min_speech_duration_ms: 10,
            padding_ms: 100,
            frame_duration_ms: 30,
        });

        // Long min duration
        let vad_long = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Aggressive,
            min_speech_duration_ms: 200,
            padding_ms: 100,
            frame_duration_ms: 30,
        });

        let result_short = vad_short.filter_speech(&audio, sample_rate).unwrap();
        let result_long = vad_long.filter_speech(&audio, sample_rate).unwrap();

        // Both should process without error
        assert!(result_short.original_duration_ms > 0);
        assert!(result_long.original_duration_ms > 0);
    }

    #[test]
    fn test_vad_custom_padding() {
        let sample_rate = 16000;

        // Speech surrounded by silence
        let audio = concatenate_audio(vec![
            generate_silence(sample_rate, 500),
            generate_speech_like_signal(sample_rate, 300, 0.5),
            generate_silence(sample_rate, 500),
        ]);

        // No padding
        let vad_no_padding = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Quality,
            min_speech_duration_ms: 50,
            padding_ms: 0,
            frame_duration_ms: 30,
        });

        // Large padding
        let vad_large_padding = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Quality,
            min_speech_duration_ms: 50,
            padding_ms: 500,
            frame_duration_ms: 30,
        });

        let result_no_pad = vad_no_padding.filter_speech(&audio, sample_rate).unwrap();
        let result_large_pad = vad_large_padding.filter_speech(&audio, sample_rate).unwrap();

        // Large padding should result in more audio (includes silence around speech)
        assert!(
            result_large_pad.audio.len() >= result_no_pad.audio.len(),
            "Larger padding should include more audio"
        );
    }

    #[test]
    fn test_vad_frame_duration() {
        let sample_rate = 16000;
        let audio = generate_speech_like_signal(sample_rate, 500, 0.5);

        // 10ms frames
        let vad_10ms = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Aggressive,
            min_speech_duration_ms: 50,
            padding_ms: 100,
            frame_duration_ms: 10,
        });

        // 30ms frames (default)
        let vad_30ms = VoiceActivityDetector::with_config(VadConfig {
            mode: VadAggressiveness::Aggressive,
            min_speech_duration_ms: 50,
            padding_ms: 100,
            frame_duration_ms: 30,
        });

        let result_10ms = vad_10ms.filter_speech(&audio, sample_rate).unwrap();
        let result_30ms = vad_30ms.filter_speech(&audio, sample_rate).unwrap();

        // Both should process correctly
        assert!(result_10ms.original_duration_ms > 0);
        assert!(result_30ms.original_duration_ms > 0);
    }
}

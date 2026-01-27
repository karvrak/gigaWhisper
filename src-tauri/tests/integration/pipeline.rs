//! Integration Tests for Audio-to-Transcription Pipeline
//!
//! Tests the complete flow: AudioCapture -> RingBuffer -> Resample -> VAD -> TranscriptionService
//!
//! These tests verify:
//! 1. Module interactions work correctly
//! 2. Error handling cascades properly
//! 3. Pipeline performance meets requirements (< 500ms latency where measurable)

use async_trait::async_trait;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// Re-export modules from the library crate
use gigawhisper_lib::audio::{
    resample, RingBuffer, VadConfig, VadAggressiveness, VoiceActivityDetector, encode_wav,
    normalize, duration_seconds, has_voice_activity,
};
use gigawhisper_lib::transcription::{
    TranscriptionConfig, TranscriptionError, TranscriptionOrchestrator, TranscriptionProvider,
    TranscriptionResult,
};

// ============================================================================
// Test Fixtures and Mock Providers
// ============================================================================

/// Generate synthetic audio samples (sine wave)
fn generate_sine_wave(frequency: f32, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * frequency * t).sin()
        })
        .collect()
}

/// Generate synthetic speech-like audio (multiple harmonics with amplitude modulation)
fn generate_speech_like_audio(sample_rate: u32, duration_secs: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            // Fundamental frequency (typical male voice ~120Hz)
            let fundamental = (2.0 * std::f32::consts::PI * 120.0 * t).sin();
            // Second harmonic
            let harmonic2 = 0.5 * (2.0 * std::f32::consts::PI * 240.0 * t).sin();
            // Third harmonic
            let harmonic3 = 0.25 * (2.0 * std::f32::consts::PI * 360.0 * t).sin();
            // Amplitude modulation (syllables ~3-4Hz)
            let envelope = 0.5 + 0.5 * (2.0 * std::f32::consts::PI * 3.5 * t).sin();

            envelope * (fundamental + harmonic2 + harmonic3) * 0.3
        })
        .collect()
}

/// Generate silence
fn generate_silence(sample_rate: u32, duration_secs: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    vec![0.0; num_samples]
}

/// Generate audio with speech surrounded by silence
fn generate_speech_with_silence(sample_rate: u32) -> Vec<f32> {
    let mut audio = Vec::new();
    // 0.5s silence
    audio.extend(generate_silence(sample_rate, 0.5));
    // 2s speech
    audio.extend(generate_speech_like_audio(sample_rate, 2.0));
    // 0.5s silence
    audio.extend(generate_silence(sample_rate, 0.5));
    audio
}

/// Mock transcription provider for testing with configurable behavior
#[allow(dead_code)]
struct MockTranscriptionProvider {
    name: &'static str,
    available: bool,
    should_fail: bool,
    fail_error: Option<TranscriptionError>,
    delay_ms: Option<u64>,
    call_count: Arc<AtomicU32>,
    received_audio_len: Arc<AtomicU32>,
}

#[allow(dead_code)]
impl MockTranscriptionProvider {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            available: true,
            should_fail: false,
            fail_error: None,
            delay_ms: None,
            call_count: Arc::new(AtomicU32::new(0)),
            received_audio_len: Arc::new(AtomicU32::new(0)),
        }
    }

    fn available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    fn failing(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn with_error(mut self, error: TranscriptionError) -> Self {
        self.should_fail = true;
        self.fail_error = Some(error);
        self
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = Some(delay_ms);
        self
    }

    fn with_call_counter(mut self, counter: Arc<AtomicU32>) -> Self {
        self.call_count = counter;
        self
    }

    fn with_audio_len_tracker(mut self, tracker: Arc<AtomicU32>) -> Self {
        self.received_audio_len = tracker;
        self
    }

    fn call_count(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }

    fn received_audio_len(&self) -> u32 {
        self.received_audio_len.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl TranscriptionProvider for MockTranscriptionProvider {
    async fn transcribe(
        &self,
        audio: &[f32],
        _config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.received_audio_len.store(audio.len() as u32, Ordering::SeqCst);

        // Simulate processing delay if configured
        if let Some(delay) = self.delay_ms {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        if self.should_fail {
            Err(self.fail_error.clone().unwrap_or_else(|| {
                TranscriptionError::Failed("Mock failure".to_string())
            }))
        } else {
            Ok(TranscriptionResult {
                text: "Transcribed text from mock provider".to_string(),
                language: Some("en".to_string()),
                duration_ms: self.delay_ms.unwrap_or(50),
                provider: self.name.to_string(),
            })
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

// ============================================================================
// SECTION 1: RingBuffer -> Resample Integration Tests
// ============================================================================

mod buffer_resample_integration {
    use super::*;

    #[test]
    fn test_buffer_to_resample_flow() {
        // Simulate audio capture at 48kHz into ring buffer, then resample to 16kHz
        let device_rate = 48000u32;
        let whisper_rate = 16000u32;

        // Create buffer for 3 seconds of audio
        let buffer_size = device_rate as usize * 3;
        let mut buffer = RingBuffer::new(buffer_size);

        // Generate 1 second of audio at device rate
        let audio = generate_sine_wave(440.0, device_rate, 1.0);

        // Write to buffer
        buffer.write(&audio);
        assert_eq!(buffer.len(), device_rate as usize);

        // Drain and resample
        let drained = buffer.drain();
        let resampled = resample(&drained, device_rate, whisper_rate).unwrap();

        // Verify resampled length (should be ~1/3 of original for 48k->16k)
        let expected_len = whisper_rate as usize; // 1 second at 16kHz
        // Allow 10% tolerance for resampler overhead
        assert!(
            (resampled.len() as f32 - expected_len as f32).abs() < expected_len as f32 * 0.1,
            "Expected ~{} samples, got {}",
            expected_len,
            resampled.len()
        );
    }

    #[test]
    fn test_buffer_overflow_then_resample() {
        // Test that overflow handling doesn't corrupt data for resampling
        let mut buffer = RingBuffer::new(1000);

        // Write more than capacity
        let audio = generate_sine_wave(440.0, 16000, 0.2); // ~3200 samples
        buffer.write(&audio);

        // Buffer should have exactly capacity samples
        assert_eq!(buffer.len(), 1000);

        // Drain and verify resampling still works
        let drained = buffer.drain();
        let resampled = resample(&drained, 16000, 16000).unwrap();

        assert_eq!(resampled.len(), 1000);
    }

    #[test]
    fn test_multiple_write_cycles_then_resample() {
        let mut buffer = RingBuffer::new(8000); // 0.5s at 16kHz

        // Simulate multiple capture cycles
        for _ in 0..10 {
            let chunk = generate_sine_wave(440.0, 16000, 0.1); // 1600 samples
            buffer.write(&chunk);
        }

        // Drain all and resample to same rate (no-op)
        let drained = buffer.drain();
        let resampled = resample(&drained, 16000, 16000).unwrap();

        assert_eq!(resampled.len(), 8000);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_resample_preserves_audio_energy() {
        let original = generate_sine_wave(440.0, 48000, 1.0);
        let resampled = resample(&original, 48000, 16000).unwrap();

        // Calculate RMS of both
        let original_rms: f32 = (original.iter().map(|s| s * s).sum::<f32>() / original.len() as f32).sqrt();
        let resampled_rms: f32 = (resampled.iter().map(|s| s * s).sum::<f32>() / resampled.len() as f32).sqrt();

        // RMS should be similar (within 20% for FFT resampler)
        let ratio = resampled_rms / original_rms;
        assert!(
            ratio > 0.8 && ratio < 1.2,
            "RMS ratio {} out of expected range [0.8, 1.2]",
            ratio
        );
    }
}

// ============================================================================
// SECTION 2: Resample -> VAD Integration Tests
// ============================================================================

mod resample_vad_integration {
    use super::*;

    #[test]
    fn test_resample_then_vad_speech_detection() {
        // Generate speech-like audio at 48kHz
        let audio_48k = generate_speech_like_audio(48000, 2.0);

        // Resample to 16kHz (VAD requirement)
        let audio_16k = resample(&audio_48k, 48000, 16000).unwrap();

        // Apply VAD
        let vad = VoiceActivityDetector::new();
        let result = vad.filter_speech(&audio_16k, 16000).unwrap();

        // Should detect speech
        assert!(result.speech_segments > 0, "Should detect speech segments");
        assert!(result.speech_percentage > 50.0, "Should have significant speech");
    }

    #[test]
    fn test_resample_then_vad_silence_detection() {
        // Generate silence at 48kHz
        let silence_48k = generate_silence(48000, 2.0);

        // Resample to 16kHz
        let silence_16k = resample(&silence_48k, 48000, 16000).unwrap();

        // Apply VAD
        let vad = VoiceActivityDetector::new();
        let result = vad.filter_speech(&silence_16k, 16000).unwrap();

        // Should detect minimal or no speech
        assert!(result.speech_percentage < 10.0, "Silence should have minimal speech detection");
    }

    #[test]
    fn test_vad_filters_silence_from_speech() {
        // Generate speech surrounded by silence at 48kHz
        let audio_48k = generate_speech_with_silence(48000);

        // Resample to 16kHz
        let audio_16k = resample(&audio_48k, 48000, 16000).unwrap();

        let original_len = audio_16k.len();

        // Apply VAD
        let vad = VoiceActivityDetector::new();
        let result = vad.filter_speech(&audio_16k, 16000).unwrap();

        // Filtered audio should be shorter (silence removed)
        assert!(
            result.audio.len() < original_len,
            "VAD should filter out silence, original: {}, filtered: {}",
            original_len,
            result.audio.len()
        );
    }

    #[test]
    fn test_vad_different_aggressiveness_levels() {
        let audio = generate_speech_like_audio(16000, 2.0);

        let mut results = Vec::new();

        for mode in [VadAggressiveness::Quality, VadAggressiveness::Aggressive, VadAggressiveness::VeryAggressive] {
            let config = VadConfig {
                mode,
                min_speech_duration_ms: 100,
                padding_ms: 300,
                frame_duration_ms: 30,
            };
            let vad = VoiceActivityDetector::with_config(config);
            let result = vad.filter_speech(&audio, 16000).unwrap();
            results.push((mode, result.speech_percentage));
        }

        // More aggressive modes should generally filter more
        // (though this depends on the audio content)
        assert!(results.len() == 3, "Should have results for all modes");
    }
}

// ============================================================================
// SECTION 3: VAD -> TranscriptionService Integration Tests
// ============================================================================

mod vad_transcription_integration {
    use super::*;

    #[tokio::test]
    async fn test_vad_filtered_audio_to_transcription() {
        // Generate speech-like audio
        let audio = generate_speech_like_audio(16000, 2.0);

        // Apply VAD
        let vad = VoiceActivityDetector::new();
        let vad_result = vad.filter_speech(&audio, 16000).unwrap();

        // Create mock provider that tracks received audio length
        let audio_len_tracker = Arc::new(AtomicU32::new(0));
        let provider = MockTranscriptionProvider::new("mock_whisper")
            .with_audio_len_tracker(audio_len_tracker.clone());

        let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

        // Transcribe the VAD-filtered audio
        let result = orchestrator
            .transcribe(&vad_result.audio, &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        assert!(audio_len_tracker.load(Ordering::SeqCst) > 0);
    }

    #[tokio::test]
    async fn test_empty_vad_result_handling() {
        // Generate silence (VAD should return empty)
        let silence = generate_silence(16000, 2.0);

        // Apply VAD
        let vad = VoiceActivityDetector::new();
        let vad_result = vad.filter_speech(&silence, 16000).unwrap();

        // Create mock provider
        let provider = MockTranscriptionProvider::new("mock_whisper");
        let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

        // Transcribe with potentially empty audio
        if vad_result.audio.is_empty() {
            let result = orchestrator
                .transcribe(&vad_result.audio, &TranscriptionConfig::default())
                .await;

            // Should return InvalidAudio error for empty input
            assert!(result.is_err());
            match result.unwrap_err() {
                TranscriptionError::InvalidAudio(msg) => assert!(msg.contains("Empty")),
                _ => panic!("Expected InvalidAudio error"),
            }
        }
    }
}

// ============================================================================
// SECTION 4: Full Pipeline Integration Tests
// ============================================================================

mod full_pipeline_integration {
    use super::*;

    #[tokio::test]
    async fn test_complete_pipeline_flow() {
        // Simulate full pipeline: capture -> buffer -> resample -> VAD -> transcribe

        // 1. Simulate audio capture at 48kHz
        let device_rate = 48000u32;
        let captured_audio = generate_speech_like_audio(device_rate, 3.0);

        // 2. Write to ring buffer
        let buffer_size = device_rate as usize * 60; // 60s buffer
        let mut buffer = RingBuffer::new(buffer_size);
        buffer.write(&captured_audio);

        // 3. Drain buffer
        let raw_samples = buffer.drain();
        assert!(!raw_samples.is_empty());

        // 4. Resample to 16kHz
        let whisper_rate = 16000u32;
        let resampled = resample(&raw_samples, device_rate, whisper_rate).unwrap();
        assert!(!resampled.is_empty());

        // 5. Apply VAD
        let vad = VoiceActivityDetector::new();
        let vad_result = vad.filter_speech(&resampled, whisper_rate).unwrap();

        // 6. Transcribe (with mock)
        let provider = MockTranscriptionProvider::new("mock_whisper");
        let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

        let result = orchestrator
            .transcribe(&vad_result.audio, &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        let transcription = result.unwrap();
        assert!(!transcription.text.is_empty());
        assert_eq!(transcription.provider, "mock_whisper");
    }

    #[tokio::test]
    async fn test_pipeline_with_different_sample_rates() {
        // Test common device sample rates
        for device_rate in [44100u32, 48000, 96000] {
            let audio = generate_speech_like_audio(device_rate, 1.0);

            // Resample to 16kHz
            let resampled = resample(&audio, device_rate, 16000).unwrap();

            // VAD
            let vad = VoiceActivityDetector::new();
            let vad_result = vad.filter_speech(&resampled, 16000).unwrap();

            // Transcribe
            let provider = MockTranscriptionProvider::new("mock");
            let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

            let result = orchestrator
                .transcribe(&vad_result.audio, &TranscriptionConfig::default())
                .await;

            assert!(result.is_ok(), "Pipeline should work with {}Hz input", device_rate);
        }
    }

    #[tokio::test]
    async fn test_pipeline_with_long_recording() {
        // Simulate a longer recording (30 seconds)
        let device_rate = 48000u32;
        let audio = generate_speech_like_audio(device_rate, 30.0);

        let mut buffer = RingBuffer::new(device_rate as usize * 60);
        buffer.write(&audio);

        let raw_samples = buffer.drain();
        let resampled = resample(&raw_samples, device_rate, 16000).unwrap();

        let vad = VoiceActivityDetector::new();
        let vad_result = vad.filter_speech(&resampled, 16000).unwrap();

        let provider = MockTranscriptionProvider::new("mock");
        let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

        let result = orchestrator
            .transcribe(&vad_result.audio, &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
    }
}

// ============================================================================
// SECTION 5: Error Cascade Tests
// ============================================================================

mod error_cascade_tests {
    use super::*;

    #[test]
    fn test_invalid_resample_rate_handled() {
        // Zero sample rate should be handled gracefully
        let audio = generate_sine_wave(440.0, 16000, 1.0);

        // Note: rubato may handle this differently, testing edge case
        // Most implementations would fail gracefully
        let result = resample(&audio, 0, 16000);
        // Should either work (if rate is normalized) or return error
        // The important thing is no panic
        let _ = result;
    }

    #[test]
    fn test_vad_unsupported_sample_rate_error() {
        let audio = generate_sine_wave(440.0, 16000, 1.0);
        let vad = VoiceActivityDetector::new();

        // 22050Hz is not supported by WebRTC VAD
        let result = vad.filter_speech(&audio, 22050);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transcription_failure_after_successful_vad() {
        let audio = generate_speech_like_audio(16000, 2.0);

        // VAD succeeds
        let vad = VoiceActivityDetector::new();
        let vad_result = vad.filter_speech(&audio, 16000).unwrap();

        // But transcription fails
        let provider = MockTranscriptionProvider::new("mock")
            .with_error(TranscriptionError::ModelNotLoaded);
        let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

        let result = orchestrator
            .transcribe(&vad_result.audio, &TranscriptionConfig::default())
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TranscriptionError::ModelNotLoaded));
    }

    #[tokio::test]
    async fn test_fallback_provider_on_primary_failure() {
        let audio = generate_speech_like_audio(16000, 2.0);

        let vad = VoiceActivityDetector::new();
        let vad_result = vad.filter_speech(&audio, 16000).unwrap();

        // Primary fails, fallback succeeds
        let primary = MockTranscriptionProvider::new("groq")
            .with_error(TranscriptionError::NetworkError("Connection failed".to_string()));
        let fallback = MockTranscriptionProvider::new("whisper_local");

        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(primary),
            Box::new(fallback),
        );

        let result = orchestrator
            .transcribe(&vad_result.audio, &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider, "whisper_local");
    }

    #[tokio::test]
    async fn test_both_providers_fail() {
        let audio = generate_speech_like_audio(16000, 2.0);

        let vad = VoiceActivityDetector::new();
        let vad_result = vad.filter_speech(&audio, 16000).unwrap();

        let primary = MockTranscriptionProvider::new("groq")
            .with_error(TranscriptionError::NetworkError("DNS failed".to_string()));
        let fallback = MockTranscriptionProvider::new("whisper")
            .with_error(TranscriptionError::ModelNotLoaded);

        let orchestrator = TranscriptionOrchestrator::with_fallback(
            Box::new(primary),
            Box::new(fallback),
        );

        let result = orchestrator
            .transcribe(&vad_result.audio, &TranscriptionConfig::default())
            .await;

        // Should fail with fallback's error
        assert!(result.is_err());
    }
}

// ============================================================================
// SECTION 6: Performance Tests
// ============================================================================

mod performance_tests {
    use super::*;

    #[test]
    fn test_buffer_write_performance() {
        let mut buffer = RingBuffer::new(16000 * 60); // 60s at 16kHz
        let chunk = generate_sine_wave(440.0, 16000, 0.1); // 100ms chunks

        let start = Instant::now();

        // Simulate 10 seconds of continuous capture (100 chunks)
        for _ in 0..100 {
            buffer.write(&chunk);
        }

        let elapsed = start.elapsed();

        // Should complete in well under 100ms (target: < 10ms)
        assert!(
            elapsed < Duration::from_millis(100),
            "Buffer writes took {:?}, expected < 100ms",
            elapsed
        );
    }

    #[test]
    fn test_resample_performance() {
        // 3 seconds of audio at 48kHz
        let audio = generate_sine_wave(440.0, 48000, 3.0);

        let start = Instant::now();
        let _resampled = resample(&audio, 48000, 16000).unwrap();
        let elapsed = start.elapsed();

        // Resampling 3s of audio should take < 500ms
        assert!(
            elapsed < Duration::from_millis(500),
            "Resampling took {:?}, expected < 500ms",
            elapsed
        );
    }

    #[test]
    fn test_vad_performance() {
        // 3 seconds of audio at 16kHz
        let audio = generate_speech_like_audio(16000, 3.0);

        let vad = VoiceActivityDetector::new();

        let start = Instant::now();
        let _result = vad.filter_speech(&audio, 16000).unwrap();
        let elapsed = start.elapsed();

        // VAD processing for 3s should take < 200ms
        assert!(
            elapsed < Duration::from_millis(200),
            "VAD processing took {:?}, expected < 200ms",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_full_pipeline_latency_without_transcription() {
        // Measure latency from raw audio to ready-for-transcription
        // (excludes actual model inference which would be mocked anyway)

        let device_rate = 48000u32;
        let audio = generate_speech_like_audio(device_rate, 3.0);

        let start = Instant::now();

        // 1. Buffer write and drain
        let mut buffer = RingBuffer::new(device_rate as usize * 60);
        buffer.write(&audio);
        let raw_samples = buffer.drain();

        // 2. Resample
        let resampled = resample(&raw_samples, device_rate, 16000).unwrap();

        // 3. VAD
        let vad = VoiceActivityDetector::new();
        let _vad_result = vad.filter_speech(&resampled, 16000).unwrap();

        let elapsed = start.elapsed();

        // Pipeline pre-processing for 3s audio should take < 500ms total
        assert!(
            elapsed < Duration::from_millis(500),
            "Pipeline pre-processing took {:?}, expected < 500ms",
            elapsed
        );
    }

    #[test]
    fn test_wav_encoding_performance() {
        // 10 seconds of audio
        let audio = generate_sine_wave(440.0, 16000, 10.0);

        let start = Instant::now();
        let _wav = encode_wav(&audio, 16000, 1);
        let elapsed = start.elapsed();

        // WAV encoding should be very fast (< 50ms for 10s)
        assert!(
            elapsed < Duration::from_millis(50),
            "WAV encoding took {:?}, expected < 50ms",
            elapsed
        );
    }
}

// ============================================================================
// SECTION 7: Data Integrity Tests
// ============================================================================

mod data_integrity_tests {
    use super::*;

    #[test]
    fn test_buffer_preserves_data_order() {
        let mut buffer = RingBuffer::new(100);

        // Write sequential values
        let input: Vec<f32> = (0..50).map(|i| i as f32).collect();
        buffer.write(&input);

        let output = buffer.drain();
        assert_eq!(output, input);
    }

    #[test]
    fn test_resample_no_data_loss_for_same_rate() {
        let audio = generate_sine_wave(440.0, 16000, 1.0);
        let resampled = resample(&audio, 16000, 16000).unwrap();

        assert_eq!(resampled, audio);
    }

    #[test]
    fn test_normalize_preserves_relative_values() {
        let mut audio = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let original = audio.clone();

        normalize(&mut audio);

        // After normalization, max should be 1.0
        let max = audio.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!((max - 1.0).abs() < 0.001);

        // Relative proportions should be preserved
        // 0.5 / 1.0 = 0.5 in original, should still be 0.5 after
        assert!((audio[2] / audio[4] - original[2] / original[4]).abs() < 0.001);
    }

    #[test]
    fn test_wav_encoding_roundtrip_values() {
        let samples = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let wav = encode_wav(&samples, 16000, 1);

        // Verify header
        assert_eq!(&wav[0..4], b"RIFF");

        // Extract and verify samples from WAV data
        let data_offset = 44;
        for (i, &expected) in samples.iter().enumerate() {
            let byte_offset = data_offset + i * 2;
            let raw_sample = i16::from_le_bytes([wav[byte_offset], wav[byte_offset + 1]]);
            let float_sample = raw_sample as f32 / 32767.0;

            // Allow for quantization error
            assert!(
                (float_sample - expected).abs() < 0.001,
                "Sample {} mismatch: expected {}, got {}",
                i, expected, float_sample
            );
        }
    }

    #[test]
    fn test_duration_calculation_accuracy() {
        let sample_rate = 16000u32;

        // 1 second
        assert!((duration_seconds(16000, sample_rate) - 1.0).abs() < 0.001);

        // 2.5 seconds
        assert!((duration_seconds(40000, sample_rate) - 2.5).abs() < 0.001);

        // 0 seconds
        assert_eq!(duration_seconds(0, sample_rate), 0.0);
    }
}

// ============================================================================
// SECTION 8: Edge Cases and Boundary Conditions
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_audio_handling() {
        let empty: Vec<f32> = vec![];

        // Resample should handle empty input
        let resampled = resample(&empty, 48000, 16000).unwrap();
        assert!(resampled.is_empty());

        // VAD should handle empty input gracefully
        let vad = VoiceActivityDetector::new();
        let result = vad.filter_speech(&empty, 16000);
        // May return error or empty result
        let _ = result;
    }

    #[tokio::test]
    async fn test_very_short_audio() {
        // 50ms of audio (below typical minimum speech duration)
        let audio = generate_sine_wave(440.0, 16000, 0.05);

        let vad = VoiceActivityDetector::new();
        let vad_result = vad.filter_speech(&audio, 16000);

        // Should not panic, may return empty or error
        let _ = vad_result;
    }

    #[test]
    fn test_audio_with_extreme_values() {
        let mut audio = vec![0.0; 100];
        audio[50] = f32::MAX;
        audio[51] = f32::MIN;

        // Normalize should clamp extreme values
        normalize(&mut audio);

        assert!(audio.iter().all(|&s| s.is_finite()));
    }

    #[test]
    fn test_buffer_single_sample() {
        let mut buffer = RingBuffer::new(100);
        buffer.write(&[0.5]);

        assert_eq!(buffer.len(), 1);
        let drained = buffer.drain();
        assert_eq!(drained, vec![0.5]);
    }

    #[test]
    fn test_buffer_exactly_at_capacity() {
        let mut buffer = RingBuffer::new(100);
        let audio = vec![0.5; 100];
        buffer.write(&audio);

        assert_eq!(buffer.len(), 100);
        assert!(!buffer.is_empty());
    }

    #[tokio::test]
    async fn test_transcription_with_single_sample() {
        let audio = vec![0.0]; // Single sample

        let provider = MockTranscriptionProvider::new("mock");
        let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

        // Should work (mock accepts any non-empty audio)
        let result = orchestrator
            .transcribe(&audio, &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_has_voice_activity_threshold_edge() {
        // Test threshold behavior
        let threshold = 0.3;

        // Audio slightly below threshold
        let below_threshold = vec![threshold - 0.05; 100];
        let rms_below = (below_threshold.iter().map(|s| s * s).sum::<f32>() / below_threshold.len() as f32).sqrt();
        assert!(!has_voice_activity(&below_threshold, threshold), "RMS {} should be below threshold {}", rms_below, threshold);

        // Audio above threshold
        let above_threshold = vec![threshold + 0.1; 100];
        let rms_above = (above_threshold.iter().map(|s| s * s).sum::<f32>() / above_threshold.len() as f32).sqrt();
        assert!(has_voice_activity(&above_threshold, threshold), "RMS {} should be above threshold {}", rms_above, threshold);
    }
}

// ============================================================================
// SECTION 9: Concurrent Access Tests
// ============================================================================

mod concurrency_tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_multiple_vad_instances_parallel() {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                thread::spawn(|| {
                    let audio = generate_speech_like_audio(16000, 1.0);
                    let vad = VoiceActivityDetector::new();
                    vad.filter_speech(&audio, 16000).unwrap()
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.original_duration_ms > 0);
        }
    }

    #[tokio::test]
    async fn test_concurrent_transcription_requests() {
        let call_counter = Arc::new(AtomicU32::new(0));

        // Create shared orchestrator
        let provider = MockTranscriptionProvider::new("mock")
            .with_call_counter(call_counter.clone())
            .with_delay(10); // Small delay to simulate processing

        let orchestrator = Arc::new(TranscriptionOrchestrator::new(Box::new(provider)));

        // Note: This test is limited because we can't easily share the orchestrator
        // In real usage, TranscriptionService handles this with interior mutability

        let audio = generate_speech_like_audio(16000, 1.0);
        let result = orchestrator
            .transcribe(&audio, &TranscriptionConfig::default())
            .await;

        assert!(result.is_ok());
        assert_eq!(call_counter.load(Ordering::SeqCst), 1);
    }
}

// ============================================================================
// SECTION 10: Language Configuration Tests
// ============================================================================

mod language_config_tests {
    use super::*;

    #[tokio::test]
    async fn test_auto_language_detection() {
        let audio = generate_speech_like_audio(16000, 2.0);

        let provider = MockTranscriptionProvider::new("mock");
        let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

        let config = TranscriptionConfig {
            language: "auto".to_string(),
            translate: false,
        };

        let result = orchestrator.transcribe(&audio, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_specific_language_config() {
        let audio = generate_speech_like_audio(16000, 2.0);

        let provider = MockTranscriptionProvider::new("mock");
        let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

        for lang in ["en", "fr", "de", "es", "ja", "zh"] {
            let config = TranscriptionConfig {
                language: lang.to_string(),
                translate: false,
            };

            let result = orchestrator.transcribe(&audio, &config).await;
            assert!(result.is_ok(), "Should handle language: {}", lang);
        }
    }

    #[tokio::test]
    async fn test_translate_mode() {
        let audio = generate_speech_like_audio(16000, 2.0);

        let provider = MockTranscriptionProvider::new("mock");
        let orchestrator = TranscriptionOrchestrator::new(Box::new(provider));

        let config = TranscriptionConfig {
            language: "fr".to_string(),
            translate: true, // Translate French to English
        };

        let result = orchestrator.transcribe(&audio, &config).await;
        assert!(result.is_ok());
    }
}

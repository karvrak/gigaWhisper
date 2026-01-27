//! Audio Format Utilities
//!
//! Conversion and encoding functions.

use rubato::{FftFixedIn, Resampler};

/// Encode samples as WAV format bytes
pub fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<u8> {
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_size = samples.len() * 2; // 16-bit samples

    let mut wav = Vec::with_capacity(44 + data_size);

    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&((36 + data_size) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_size as u32).to_le_bytes());

    // Convert f32 samples to i16
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let i16_sample = (clamped * 32767.0) as i16;
        wav.extend_from_slice(&i16_sample.to_le_bytes());
    }

    wav
}

/// Calculate audio duration from sample count
pub fn duration_seconds(sample_count: usize, sample_rate: u32) -> f32 {
    sample_count as f32 / sample_rate as f32
}

/// Normalize audio samples to -1.0 to 1.0 range
pub fn normalize(samples: &mut [f32]) {
    let max = samples
        .iter()
        .map(|s| s.abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(1.0);

    if max > 0.0 && max != 1.0 {
        let scale = 1.0 / max;
        for sample in samples.iter_mut() {
            *sample *= scale;
        }
    }
}

/// Resample audio from source sample rate to target sample rate (16kHz for Whisper)
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>, String> {
    if from_rate == to_rate {
        return Ok(samples.to_vec());
    }

    if samples.is_empty() {
        return Ok(Vec::new());
    }

    // Calculate chunk size (must be power of 2 for FFT resampler)
    let chunk_size = 1024;

    // Create resampler
    let mut resampler = FftFixedIn::<f32>::new(
        from_rate as usize,
        to_rate as usize,
        chunk_size,
        2, // sub_chunks
        1, // channels (mono)
    )
    .map_err(|e| format!("Failed to create resampler: {}", e))?;

    // Process in chunks
    let mut output = Vec::new();
    let mut pos = 0;

    while pos < samples.len() {
        let end = (pos + chunk_size).min(samples.len());
        let mut chunk = samples[pos..end].to_vec();

        // Pad last chunk if needed
        if chunk.len() < chunk_size {
            chunk.resize(chunk_size, 0.0);
        }

        // Resample (input is Vec of channels, each channel is a Vec of samples)
        let input = vec![chunk];
        match resampler.process(&input, None) {
            Ok(resampled) => {
                if !resampled.is_empty() && !resampled[0].is_empty() {
                    output.extend_from_slice(&resampled[0]);
                }
            }
            Err(e) => {
                tracing::warn!("Resampling error: {}", e);
            }
        }

        pos += chunk_size;
    }

    tracing::info!(
        "Resampled {} samples ({}Hz) to {} samples ({}Hz)",
        samples.len(),
        from_rate,
        output.len(),
        to_rate
    );

    Ok(output)
}

/// Simple voice activity detection
/// Returns true if audio contains speech-like content
pub fn has_voice_activity(samples: &[f32], threshold: f32) -> bool {
    if samples.is_empty() {
        return false;
    }

    // Calculate RMS energy
    let rms: f32 = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

    rms > threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_wav() {
        let samples = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let wav = encode_wav(&samples, 16000, 1);

        // Check RIFF header
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
    }

    #[test]
    fn test_normalize() {
        let mut samples = vec![0.0, 0.25, -0.5];
        normalize(&mut samples);

        assert_eq!(samples[0], 0.0);
        assert_eq!(samples[1], 0.5);
        assert_eq!(samples[2], -1.0);
    }

    #[test]
    fn test_has_voice_activity() {
        let silence = vec![0.0; 100];
        let voice = vec![0.3; 100];

        assert!(!has_voice_activity(&silence, 0.1));
        assert!(has_voice_activity(&voice, 0.1));
    }

    #[test]
    fn test_has_voice_activity_empty() {
        let empty: Vec<f32> = vec![];
        assert!(!has_voice_activity(&empty, 0.1));
    }

    #[test]
    fn test_has_voice_activity_edge_cases() {
        // Very low threshold
        let low_signal = vec![0.01; 100];
        assert!(has_voice_activity(&low_signal, 0.001));
        assert!(!has_voice_activity(&low_signal, 0.1));

        // Mixed signal
        let mixed = vec![0.0, 0.5, 0.0, 0.5, 0.0];
        let rms = (mixed.iter().map(|s| s * s).sum::<f32>() / mixed.len() as f32).sqrt();
        assert!(has_voice_activity(&mixed, rms - 0.01));
    }

    #[test]
    fn test_duration_seconds() {
        assert_eq!(duration_seconds(16000, 16000), 1.0);
        assert_eq!(duration_seconds(8000, 16000), 0.5);
        assert_eq!(duration_seconds(48000, 16000), 3.0);
        assert_eq!(duration_seconds(0, 16000), 0.0);
    }

    #[test]
    fn test_normalize_empty() {
        let mut samples: Vec<f32> = vec![];
        normalize(&mut samples);
        assert!(samples.is_empty());
    }

    #[test]
    fn test_normalize_already_normalized() {
        let mut samples = vec![-1.0, 0.0, 1.0];
        normalize(&mut samples);
        assert_eq!(samples[0], -1.0);
        assert_eq!(samples[1], 0.0);
        assert_eq!(samples[2], 1.0);
    }

    #[test]
    fn test_normalize_silent() {
        let mut samples = vec![0.0, 0.0, 0.0];
        normalize(&mut samples);
        // Should remain unchanged (or at least not crash)
        assert_eq!(samples, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_encode_wav_header_structure() {
        let samples = vec![0.0; 100];
        let wav = encode_wav(&samples, 16000, 1);

        // Check file size (44 byte header + 200 bytes of data)
        assert_eq!(wav.len(), 44 + 200);

        // Check RIFF chunk
        assert_eq!(&wav[0..4], b"RIFF");

        // Check file size field (total size - 8)
        let file_size = u32::from_le_bytes([wav[4], wav[5], wav[6], wav[7]]);
        assert_eq!(file_size as usize, wav.len() - 8);

        // Check WAVE format
        assert_eq!(&wav[8..12], b"WAVE");

        // Check fmt chunk
        assert_eq!(&wav[12..16], b"fmt ");

        // Check fmt chunk size (16 for PCM)
        let fmt_size = u32::from_le_bytes([wav[16], wav[17], wav[18], wav[19]]);
        assert_eq!(fmt_size, 16);

        // Check audio format (1 = PCM)
        let audio_format = u16::from_le_bytes([wav[20], wav[21]]);
        assert_eq!(audio_format, 1);

        // Check number of channels
        let num_channels = u16::from_le_bytes([wav[22], wav[23]]);
        assert_eq!(num_channels, 1);

        // Check sample rate
        let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        assert_eq!(sample_rate, 16000);

        // Check data chunk
        assert_eq!(&wav[36..40], b"data");
    }

    #[test]
    fn test_encode_wav_sample_values() {
        // Test that samples are correctly converted to 16-bit PCM
        let samples = vec![0.0, 1.0, -1.0, 0.5, -0.5];
        let wav = encode_wav(&samples, 16000, 1);

        // Data starts at offset 44
        let data_offset = 44;

        // Sample 0: 0.0 -> 0
        let s0 = i16::from_le_bytes([wav[data_offset], wav[data_offset + 1]]);
        assert_eq!(s0, 0);

        // Sample 1: 1.0 -> 32767
        let s1 = i16::from_le_bytes([wav[data_offset + 2], wav[data_offset + 3]]);
        assert_eq!(s1, 32767);

        // Sample 2: -1.0 -> -32767
        let s2 = i16::from_le_bytes([wav[data_offset + 4], wav[data_offset + 5]]);
        assert_eq!(s2, -32767);
    }

    #[test]
    fn test_resample_same_rate() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = resample(&samples, 16000, 16000).unwrap();
        assert_eq!(result, samples);
    }

    #[test]
    fn test_resample_empty() {
        let samples: Vec<f32> = vec![];
        let result = resample(&samples, 44100, 16000).unwrap();
        assert!(result.is_empty());
    }
}

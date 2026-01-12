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
        .max_by(|a, b| a.partial_cmp(b).unwrap())
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
}

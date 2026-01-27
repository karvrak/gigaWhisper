//! Transcription Service
//!
//! Centralized service for managing transcription operations.
//! Handles provider caching, status tracking, and shared logic.

use super::{GroqProvider, TranscriptionConfig, TranscriptionProvider, TranscriptionResult, WhisperProvider};
use crate::audio::{resample, VadAggressiveness, VadConfig, VoiceActivityDetector};
use crate::config::{Settings, TranscriptionProvider as ConfigProvider};
use crate::output;
use crate::utils::{metrics, TranscriptionRecord};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// Transcription status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptionStatus {
    pub provider: String,
    pub model: String,
    pub model_loaded: bool,
    pub is_transcribing: bool,
    pub last_result: Option<String>,
    pub last_duration_ms: Option<u64>,
    pub last_error: Option<String>,
}

impl Default for TranscriptionStatus {
    fn default() -> Self {
        Self {
            provider: "local".to_string(),
            model: "base".to_string(),
            model_loaded: false,
            is_transcribing: false,
            last_result: None,
            last_duration_ms: None,
            last_error: None,
        }
    }
}

/// Cached Whisper provider data
struct CachedWhisper {
    provider: WhisperProvider,
    model_path: PathBuf,
    gpu_enabled: bool,
    threads: usize,
}

/// Centralized transcription service
pub struct TranscriptionService {
    /// Cached Whisper provider
    cached_whisper: RwLock<Option<CachedWhisper>>,
    /// Transcription status
    status: RwLock<TranscriptionStatus>,
}

impl TranscriptionService {
    /// Create a new transcription service
    pub fn new() -> Self {
        Self {
            cached_whisper: RwLock::new(None),
            status: RwLock::new(TranscriptionStatus::default()),
        }
    }

    /// Get current transcription status
    pub fn get_status(&self) -> TranscriptionStatus {
        self.status.read().clone()
    }

    /// Update status with current config
    pub fn update_status_from_config(&self, config: &Settings) {
        let mut status = self.status.write();
        status.provider = match config.transcription.provider {
            ConfigProvider::Local => "local".to_string(),
            ConfigProvider::Groq => "groq".to_string(),
        };
        status.model = format!("{:?}", config.transcription.local.model).to_lowercase();

        // Check if model is loaded
        let cached = self.cached_whisper.read();
        status.model_loaded = cached.as_ref().map(|c| c.provider.is_model_loaded()).unwrap_or(false);
    }

    /// Preload the Whisper model (call during startup or settings change)
    pub fn preload_model(&self, config: &Settings) -> Result<(), String> {
        if config.transcription.provider == ConfigProvider::Local {
            let model_path = crate::config::models_dir()
                .join(config.transcription.local.model_filename());
            let threads = config.transcription.local.threads;
            let gpu_enabled = config.transcription.local.gpu_enabled;

            self.ensure_whisper_loaded(model_path, threads, gpu_enabled)?;
        }
        Ok(())
    }

    /// Ensure Whisper model is loaded (with caching)
    fn ensure_whisper_loaded(
        &self,
        model_path: PathBuf,
        threads: usize,
        gpu_enabled: bool,
    ) -> Result<(), String> {
        let needs_load = {
            let cached = self.cached_whisper.read();
            match &*cached {
                Some(c) => {
                    c.model_path != model_path
                        || c.gpu_enabled != gpu_enabled
                        || c.threads != threads
                        || !c.provider.is_model_loaded()
                }
                None => true,
            }
        };

        if needs_load {
            tracing::info!(
                "Loading Whisper model: {:?} (GPU: {}, threads: {})",
                model_path,
                if gpu_enabled { "enabled" } else { "disabled" },
                threads
            );

            let provider = WhisperProvider::with_gpu(model_path.clone(), threads, gpu_enabled);
            provider.load_model().map_err(|e| e.to_string())?;

            let mut cached = self.cached_whisper.write();
            *cached = Some(CachedWhisper {
                provider,
                model_path,
                gpu_enabled,
                threads,
            });

            let mut status = self.status.write();
            status.model_loaded = true;
        }

        Ok(())
    }

    /// Unload the model to free memory
    pub fn unload_model(&self) {
        let mut cached = self.cached_whisper.write();
        if let Some(c) = cached.as_ref() {
            c.provider.unload_model();
        }
        *cached = None;

        let mut status = self.status.write();
        status.model_loaded = false;
    }

    /// Perform transcription with the configured provider
    pub async fn transcribe(
        &self,
        samples: &[f32],
        config: &Settings,
    ) -> Result<TranscriptionResult, String> {
        // Update status
        {
            let mut status = self.status.write();
            status.is_transcribing = true;
            status.last_error = None;
        }

        let transcription_config = TranscriptionConfig {
            language: config.transcription.language.clone(),
            translate: false,
        };

        let result = match config.transcription.provider {
            ConfigProvider::Groq => {
                let provider = GroqProvider::with_timeout(
                    Some(config.transcription.groq.model.clone()),
                    config.transcription.groq.timeout_seconds as u64,
                );
                provider
                    .transcribe(samples, &transcription_config)
                    .await
                    .map_err(|e| e.to_string())
            }
            ConfigProvider::Local => {
                let model_path = crate::config::models_dir()
                    .join(config.transcription.local.model_filename());
                let threads = config.transcription.local.threads;
                let gpu_enabled = config.transcription.local.gpu_enabled;

                // Ensure model is loaded and get a clone of the provider
                self.ensure_whisper_loaded(model_path, threads, gpu_enabled)?;

                // Get a clone of the cached provider (cheap because context is Arc)
                let provider = {
                    let cached = self.cached_whisper.read();
                    cached.as_ref().ok_or("Provider not initialized")?.provider.clone()
                };

                // Transcribe using the cloned provider (no lock held across await)
                provider
                    .transcribe(samples, &transcription_config)
                    .await
                    .map_err(|e| e.to_string())
            }
        };

        // Update status with result
        {
            let mut status = self.status.write();
            status.is_transcribing = false;
            match &result {
                Ok(r) => {
                    status.last_result = Some(r.text.clone());
                    status.last_duration_ms = Some(r.duration_ms);
                    status.last_error = None;
                }
                Err(e) => {
                    status.last_error = Some(e.clone());
                }
            }
        }

        result
    }

    /// Process recording: resample, apply VAD, transcribe, and output
    pub async fn process_recording(
        self: &Arc<Self>,
        app: &AppHandle,
        raw_samples: Vec<f32>,
        device_sample_rate: u32,
    ) -> Result<String, String> {
        use tauri_plugin_notification::NotificationExt;

        let state = app.state::<crate::AppState>();

        // Resample to 16kHz for Whisper
        const WHISPER_SAMPLE_RATE: u32 = 16000;
        let samples = if device_sample_rate != WHISPER_SAMPLE_RATE {
            resample(&raw_samples, device_sample_rate, WHISPER_SAMPLE_RATE)
                .map_err(|e| format!("Resampling failed: {}", e))?
        } else {
            raw_samples
        };

        // Check for minimum audio
        if samples.len() < 1600 {
            return Err("Recording too short".to_string());
        }

        // Get config
        let config = state.config.read().clone();

        // Apply Voice Activity Detection if enabled
        let samples_for_transcription = if config.audio.vad.enabled {
            let vad_mode = match config.audio.vad.aggressiveness {
                0 => VadAggressiveness::Quality,
                1 => VadAggressiveness::LowBitrate,
                2 => VadAggressiveness::Aggressive,
                _ => VadAggressiveness::VeryAggressive,
            };

            let vad_config = VadConfig {
                mode: vad_mode,
                min_speech_duration_ms: config.audio.vad.min_speech_duration_ms,
                padding_ms: config.audio.vad.padding_ms,
                frame_duration_ms: 30,
            };

            let vad = VoiceActivityDetector::with_config(vad_config);

            match vad.filter_speech(&samples, WHISPER_SAMPLE_RATE) {
                Ok(vad_result) => {
                    tracing::info!(
                        "VAD filtered: {:.1}% speech ({} segments), {}ms -> {}ms",
                        vad_result.speech_percentage,
                        vad_result.speech_segments,
                        vad_result.original_duration_ms,
                        vad_result.speech_duration_ms
                    );

                    // If no speech detected, return early
                    if vad_result.audio.is_empty() || vad_result.speech_percentage < 1.0 {
                        return Err("No speech detected in recording".to_string());
                    }

                    vad_result.audio
                }
                Err(e) => {
                    tracing::warn!("VAD failed, using full audio: {}", e);
                    samples.clone()
                }
            }
        } else {
            samples.clone()
        };

        // Calculate audio durations for metrics
        let original_audio_ms = (samples.len() as u64 * 1000) / WHISPER_SAMPLE_RATE as u64;
        let filtered_audio_ms = (samples_for_transcription.len() as u64 * 1000) / WHISPER_SAMPLE_RATE as u64;
        let vad_was_enabled = config.audio.vad.enabled;

        // Perform transcription
        let result = self.transcribe(&samples_for_transcription, &config).await;

        match result {
            Ok(transcription) => {
                let text = transcription.text.clone();
                tracing::info!(
                    "Transcription complete: '{}' ({}ms, {})",
                    text,
                    transcription.duration_ms,
                    transcription.provider
                );

                // Record performance metrics
                let record = TranscriptionRecord::builder()
                    .audio_duration_ms(original_audio_ms)
                    .processing_time_ms(transcription.duration_ms)
                    .provider(&transcription.provider)
                    .model(format!("{:?}", config.transcription.local.model).to_lowercase())
                    .gpu_used(config.transcription.local.gpu_enabled)
                    .threads_used(config.transcription.local.threads)
                    .vad_enabled(vad_was_enabled)
                    .vad_filtered_ms(filtered_audio_ms)
                    .result_chars(text.len())
                    .build();
                metrics().write().record_transcription(record);

                // Save to history with audio (only if not empty)
                if !text.is_empty() {
                    crate::history::add_transcription_with_audio(
                        text.clone(),
                        transcription.duration_ms,
                        transcription.provider.clone(),
                        transcription.language.clone(),
                        &samples,
                        WHISPER_SAMPLE_RATE,
                    );
                    let _ = app.emit("history:updated", ());
                }

                // Output the text
                if let Err(e) = self.output_text(&text, app).await {
                    tracing::error!("Failed to output text: {}", e);
                }

                // Emit success event
                let _ = app.emit("transcription:complete", &text);

                // Notify user
                let preview = if text.len() > 50 {
                    format!("{}...", &text[..50])
                } else if text.is_empty() {
                    "(No speech detected)".to_string()
                } else {
                    text.clone()
                };
                let _ = app
                    .notification()
                    .builder()
                    .title("Transcription Complete")
                    .body(&preview)
                    .show();

                Ok(text)
            }
            Err(e) => {
                tracing::error!("Transcription failed: {}", e);
                let _ = app.emit("transcription:error", &e);

                let _ = app
                    .notification()
                    .builder()
                    .title("Transcription Failed")
                    .body(&e)
                    .show();

                Err(e)
            }
        }
    }

    /// Output transcribed text (clipboard + paste or popup)
    async fn output_text(&self, text: &str, app: &AppHandle) -> Result<(), String> {
        if text.is_empty() {
            tracing::info!("Empty transcription, nothing to output");
            return Ok(());
        }

        let should_paste = output::should_auto_paste();

        if should_paste {
            output::copy_to_clipboard(text)
                .map_err(|e| format!("Clipboard error: {}", e))?;

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            output::send_ctrl_v()
                .map_err(|e| format!("Keyboard error: {}", e))?;

            tracing::info!("Text pasted to active window");
        } else {
            let _ = output::copy_to_clipboard(text);
            let _ = app.emit("show:popup", text);
            tracing::info!("Showing popup (GigaWhisper is active window)");
        }

        Ok(())
    }
}

impl Default for TranscriptionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // TranscriptionStatus Tests
    // ============================================================

    #[test]
    fn test_transcription_status_default() {
        let status = TranscriptionStatus::default();

        assert_eq!(status.provider, "local");
        assert_eq!(status.model, "base");
        assert!(!status.model_loaded);
        assert!(!status.is_transcribing);
        assert!(status.last_result.is_none());
        assert!(status.last_duration_ms.is_none());
        assert!(status.last_error.is_none());
    }

    #[test]
    fn test_transcription_status_clone() {
        let mut status = TranscriptionStatus::default();
        status.provider = "groq".to_string();
        status.model = "whisper-large-v3".to_string();
        status.model_loaded = true;
        status.is_transcribing = true;
        status.last_result = Some("Hello world".to_string());
        status.last_duration_ms = Some(1500);
        status.last_error = None;

        let cloned = status.clone();

        assert_eq!(cloned.provider, "groq");
        assert_eq!(cloned.model, "whisper-large-v3");
        assert!(cloned.model_loaded);
        assert!(cloned.is_transcribing);
        assert_eq!(cloned.last_result, Some("Hello world".to_string()));
        assert_eq!(cloned.last_duration_ms, Some(1500));
        assert!(cloned.last_error.is_none());
    }

    #[test]
    fn test_transcription_status_serialization() {
        let status = TranscriptionStatus::default();
        let json = serde_json::to_string(&status).unwrap();

        assert!(json.contains("provider"));
        assert!(json.contains("model"));
        assert!(json.contains("model_loaded"));
        assert!(json.contains("is_transcribing"));
    }

    #[test]
    fn test_transcription_status_with_error() {
        let mut status = TranscriptionStatus::default();
        status.last_error = Some("Model not found".to_string());

        assert_eq!(status.last_error, Some("Model not found".to_string()));
    }

    // ============================================================
    // TranscriptionService Creation Tests
    // ============================================================

    #[test]
    fn test_service_creation() {
        let service = TranscriptionService::new();
        let status = service.get_status();

        assert_eq!(status.provider, "local");
        assert_eq!(status.model, "base");
        assert!(!status.model_loaded);
        assert!(!status.is_transcribing);
    }

    #[test]
    fn test_service_default() {
        let service = TranscriptionService::default();
        let status = service.get_status();

        assert_eq!(status.provider, "local");
        assert!(!status.model_loaded);
    }

    // ============================================================
    // Status Management Tests
    // ============================================================

    #[test]
    fn test_get_status_returns_clone() {
        let service = TranscriptionService::new();

        let status1 = service.get_status();
        let status2 = service.get_status();

        // Both should be independent clones
        assert_eq!(status1.provider, status2.provider);
        assert_eq!(status1.model_loaded, status2.model_loaded);
    }

    #[test]
    fn test_unload_model_updates_status() {
        let service = TranscriptionService::new();

        // Initially not loaded
        assert!(!service.get_status().model_loaded);

        // Unload (should not panic even if not loaded)
        service.unload_model();

        // Should still report not loaded
        assert!(!service.get_status().model_loaded);
    }

    // ============================================================
    // Model Management Tests
    // ============================================================

    #[test]
    fn test_unload_model_when_not_loaded() {
        let service = TranscriptionService::new();

        // Should not panic
        service.unload_model();
        service.unload_model();
        service.unload_model();

        assert!(!service.get_status().model_loaded);
    }

    // ============================================================
    // Thread Safety Tests
    // ============================================================

    #[test]
    fn test_service_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TranscriptionService>();
    }

    #[test]
    fn test_status_can_be_shared_across_threads() {
        use std::sync::Arc;
        use std::thread;

        let service = Arc::new(TranscriptionService::new());
        let service_clone = Arc::clone(&service);

        let handle = thread::spawn(move || {
            let status = service_clone.get_status();
            assert_eq!(status.provider, "local");
        });

        let status = service.get_status();
        assert_eq!(status.provider, "local");

        handle.join().unwrap();
    }

    #[test]
    fn test_concurrent_status_reads() {
        use std::sync::Arc;
        use std::thread;

        let service = Arc::new(TranscriptionService::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let service_clone = Arc::clone(&service);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _status = service_clone.get_status();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    // ============================================================
    // Edge Cases
    // ============================================================

    #[test]
    fn test_status_with_empty_strings() {
        let mut status = TranscriptionStatus::default();
        status.provider = "".to_string();
        status.model = "".to_string();
        status.last_result = Some("".to_string());
        status.last_error = Some("".to_string());

        assert_eq!(status.provider, "");
        assert_eq!(status.model, "");
        assert_eq!(status.last_result, Some("".to_string()));
        assert_eq!(status.last_error, Some("".to_string()));
    }

    #[test]
    fn test_status_with_unicode() {
        let mut status = TranscriptionStatus::default();
        status.provider = "local".to_string();
        status.last_result = Some("Bonjour le monde".to_string());

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Bonjour"));
    }

    #[test]
    fn test_status_with_long_text() {
        let mut status = TranscriptionStatus::default();
        let long_text = "a".repeat(10000);
        status.last_result = Some(long_text.clone());

        assert_eq!(status.last_result.as_ref().unwrap().len(), 10000);
    }

    #[test]
    fn test_status_debug_format() {
        let status = TranscriptionStatus::default();
        let debug_str = format!("{:?}", status);

        assert!(debug_str.contains("TranscriptionStatus"));
        assert!(debug_str.contains("provider"));
        assert!(debug_str.contains("model"));
    }

    // ============================================================
    // Serialization Tests
    // ============================================================

    #[test]
    fn test_status_json_roundtrip() {
        let mut status = TranscriptionStatus::default();
        status.provider = "groq".to_string();
        status.model = "whisper-large-v3".to_string();
        status.model_loaded = true;
        status.is_transcribing = false;
        status.last_result = Some("Test result".to_string());
        status.last_duration_ms = Some(2500);
        status.last_error = None;

        let json = serde_json::to_string(&status).unwrap();

        // Verify all fields are present
        assert!(json.contains("\"provider\":\"groq\""));
        assert!(json.contains("\"model\":\"whisper-large-v3\""));
        assert!(json.contains("\"model_loaded\":true"));
        assert!(json.contains("\"is_transcribing\":false"));
        assert!(json.contains("\"last_result\":\"Test result\""));
        assert!(json.contains("\"last_duration_ms\":2500"));
        assert!(json.contains("\"last_error\":null"));
    }

    #[test]
    fn test_status_json_with_nulls() {
        let status = TranscriptionStatus::default();
        let json = serde_json::to_string(&status).unwrap();

        assert!(json.contains("\"last_result\":null"));
        assert!(json.contains("\"last_duration_ms\":null"));
        assert!(json.contains("\"last_error\":null"));
    }
}

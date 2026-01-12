//! Transcription Service
//!
//! Centralized service for managing transcription operations.
//! Handles provider caching, status tracking, and shared logic.

use super::{GroqProvider, TranscriptionConfig, TranscriptionProvider, TranscriptionResult, WhisperProvider};
use crate::audio::resample;
use crate::config::{Settings, TranscriptionProvider as ConfigProvider};
use crate::output;
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
                .join(config.transcription.local.model.filename());
            let threads = config.transcription.local.threads;

            self.ensure_whisper_loaded(model_path, threads)?;
        }
        Ok(())
    }

    /// Ensure Whisper model is loaded (with caching)
    fn ensure_whisper_loaded(&self, model_path: PathBuf, threads: usize) -> Result<(), String> {
        let needs_load = {
            let cached = self.cached_whisper.read();
            match &*cached {
                Some(c) => c.model_path != model_path || !c.provider.is_model_loaded(),
                None => true,
            }
        };

        if needs_load {
            tracing::info!("Loading Whisper model: {:?}", model_path);

            let provider = WhisperProvider::new(model_path.clone(), threads);
            provider.load_model().map_err(|e| e.to_string())?;

            let mut cached = self.cached_whisper.write();
            *cached = Some(CachedWhisper {
                provider,
                model_path,
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
                let provider = GroqProvider::new(Some(config.transcription.groq.model.clone()));
                provider
                    .transcribe(samples, &transcription_config)
                    .await
                    .map_err(|e| e.to_string())
            }
            ConfigProvider::Local => {
                let model_path = crate::config::models_dir()
                    .join(config.transcription.local.model.filename());
                let threads = config.transcription.local.threads;

                // Ensure model is loaded and get a clone of the provider
                self.ensure_whisper_loaded(model_path, threads)?;

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

    /// Process recording: resample, transcribe, and output
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

        // Perform transcription
        let result = self.transcribe(&samples, &config).await;

        match result {
            Ok(transcription) => {
                let text = transcription.text.clone();
                tracing::info!(
                    "Transcription complete: '{}' ({}ms, {})",
                    text,
                    transcription.duration_ms,
                    transcription.provider
                );

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

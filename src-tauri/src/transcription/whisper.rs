//! Whisper.cpp Provider
//!
//! Local transcription using whisper-rs bindings.

use super::{TranscriptionConfig, TranscriptionError, TranscriptionProvider, TranscriptionResult};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Whisper.cpp transcription provider
pub struct WhisperProvider {
    model_path: PathBuf,
    context: Arc<Mutex<Option<whisper_rs::WhisperContext>>>,
    threads: usize,
}

impl WhisperProvider {
    /// Create a new Whisper provider
    pub fn new(model_path: PathBuf, threads: usize) -> Self {
        Self {
            model_path,
            context: Arc::new(Mutex::new(None)),
            threads,
        }
    }

    /// Load the model into memory
    pub fn load_model(&self) -> Result<(), TranscriptionError> {
        if !self.model_path.exists() {
            return Err(TranscriptionError::ModelNotFound(
                self.model_path.display().to_string(),
            ));
        }

        // Check if already loaded
        if self.is_model_loaded() {
            return Ok(());
        }

        let ctx = whisper_rs::WhisperContext::new_with_params(
            self.model_path.to_str().unwrap(),
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        *self.context.lock() = Some(ctx);
        tracing::info!("Whisper model loaded: {:?}", self.model_path);

        Ok(())
    }

    /// Unload the model from memory
    pub fn unload_model(&self) {
        *self.context.lock() = None;
        tracing::info!("Whisper model unloaded");
    }

    /// Check if model is loaded
    pub fn is_model_loaded(&self) -> bool {
        self.context.lock().is_some()
    }

    /// Perform transcription synchronously (for use in spawn_blocking)
    fn transcribe_sync(
        context: Arc<Mutex<Option<whisper_rs::WhisperContext>>>,
        audio: Vec<f32>,
        config: TranscriptionConfig,
        threads: usize,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let start = Instant::now();

        let guard = context.lock();
        let ctx = guard
            .as_ref()
            .ok_or(TranscriptionError::ModelNotLoaded)?;

        // Create state for this transcription
        let mut state = ctx
            .create_state()
            .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        // Configure parameters
        let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy {
            best_of: 1,
        });

        params.set_n_threads(threads as i32);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Set language if specified
        if config.language != "auto" {
            params.set_language(Some(&config.language));
        }

        params.set_translate(config.translate);

        // Run inference
        state
            .full(params, &audio)
            .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        // Extract text from all segments
        let num_segments = state
            .full_n_segments()
            .map_err(|e| TranscriptionError::Failed(e.to_string()))?;

        let mut text = String::new();
        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                text.push_str(&segment);
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult {
            text: text.trim().to_string(),
            language: None,
            duration_ms,
            provider: "whisper.cpp".to_string(),
        })
    }
}

// Implement Clone for use with spawn_blocking
impl Clone for WhisperProvider {
    fn clone(&self) -> Self {
        Self {
            model_path: self.model_path.clone(),
            context: self.context.clone(),
            threads: self.threads,
        }
    }
}

#[async_trait]
impl TranscriptionProvider for WhisperProvider {
    async fn transcribe(
        &self,
        audio: &[f32],
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        // Ensure model is loaded
        if !self.is_model_loaded() {
            self.load_model()?;
        }

        // Clone data for the blocking task
        let context = self.context.clone();
        let audio_vec = audio.to_vec();
        let config_clone = config.clone();
        let threads = self.threads;

        // Run transcription in blocking thread pool
        // This avoids holding the MutexGuard across an await point
        tokio::task::spawn_blocking(move || {
            Self::transcribe_sync(context, audio_vec, config_clone, threads)
        })
        .await
        .map_err(|e| TranscriptionError::Failed(format!("Task failed: {}", e)))?
    }

    fn name(&self) -> &'static str {
        "whisper.cpp"
    }

    fn is_available(&self) -> bool {
        self.model_path.exists()
    }
}

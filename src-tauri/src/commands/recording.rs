//! Recording Commands
//!
//! Handle audio recording start/stop operations.

use crate::audio::{AudioCapture, AudioConfig};
use crate::{AppState, RecordingState};
use tauri::{Emitter, State};
use tauri_plugin_notification::NotificationExt;

/// Start recording audio from the microphone
#[tauri::command]
pub async fn start_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Starting recording");

    // Check current state
    {
        let recording_state = state.recording_state.read();
        match &*recording_state {
            RecordingState::Recording { .. } => {
                return Err("Already recording".to_string());
            }
            RecordingState::Processing => {
                return Err("Processing previous recording".to_string());
            }
            _ => {}
        }
    }

    // Get max_duration from config to size the buffer appropriately
    let max_duration = {
        let config = state.config.read();
        config.recording.max_duration
    };

    // Initialize audio capture with appropriate buffer size
    let audio_config = AudioConfig {
        buffer_duration_ms: max_duration * 1000, // Convert to ms
        ..AudioConfig::default()
    };
    let audio_capture = AudioCapture::new(audio_config)
        .map_err(|e| format!("Failed to initialize audio: {}", e))?;

    // Start capture
    audio_capture
        .start()
        .map_err(|e| format!("Failed to start audio capture: {}", e))?;

    // Store capture handle
    *state.audio_capture.lock() = Some(audio_capture);

    // Update state
    *state.recording_state.write() = RecordingState::Recording {
        started_at: std::time::Instant::now(),
    };

    // Notify user
    let _ = app
        .notification()
        .builder()
        .title("Recording Started")
        .body("Speak now... Press shortcut again to stop.")
        .show();

    tracing::info!("Recording started");
    Ok(())
}

/// Stop recording and trigger transcription
#[tauri::command]
pub async fn stop_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    tracing::info!("Stopping recording");

    // Get audio samples with device sample rate
    let (raw_samples, device_sample_rate) = {
        let mut capture_guard = state.audio_capture.lock();
        match capture_guard.as_ref() {
            Some(capture) => {
                let result = capture
                    .stop()
                    .map_err(|e| format!("Failed to stop audio: {}", e))?;
                *capture_guard = None;
                result
            }
            None => {
                return Err("Not recording".to_string());
            }
        }
    };

    // Check duration
    let duration = {
        let recording_state = state.recording_state.read();
        match &*recording_state {
            RecordingState::Recording { started_at } => started_at.elapsed(),
            _ => return Err("Not recording".to_string()),
        }
    };

    tracing::info!(
        "Recording stopped: {} samples, {:.2}s duration",
        raw_samples.len(),
        duration.as_secs_f32()
    );

    // Update state to processing
    *state.recording_state.write() = RecordingState::Processing;
    let _ = app.emit("recording:processing", ());

    // Use transcription service
    let service = state.transcription_service.clone();
    let result = service
        .process_recording(&app, raw_samples, device_sample_rate)
        .await;

    // Update state based on result
    match &result {
        Ok(_) => {
            *state.recording_state.write() = RecordingState::Idle;
        }
        Err(e) => {
            *state.recording_state.write() = RecordingState::Error(e.clone());
        }
    }

    result
}

/// Cancel recording without transcribing
#[tauri::command]
pub async fn cancel_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Cancelling recording");

    // Stop and discard audio
    {
        let mut capture_guard = state.audio_capture.lock();
        if let Some(capture) = capture_guard.take() {
            let _ = capture.stop();
        }
    }

    // Reset state
    *state.recording_state.write() = RecordingState::Idle;

    // Notify user
    let _ = app
        .notification()
        .builder()
        .title("Recording Cancelled")
        .body("Recording was cancelled")
        .show();

    Ok(())
}

/// Get current recording state for UI
#[tauri::command]
pub fn get_recording_state(state: State<'_, AppState>) -> RecordingStateDto {
    let recording_state = state.recording_state.read();

    match &*recording_state {
        RecordingState::Idle => RecordingStateDto {
            state: "idle".to_string(),
            duration_ms: None,
            error: None,
        },
        RecordingState::Recording { started_at } => RecordingStateDto {
            state: "recording".to_string(),
            duration_ms: Some(started_at.elapsed().as_millis() as u64),
            error: None,
        },
        RecordingState::Processing => RecordingStateDto {
            state: "processing".to_string(),
            duration_ms: None,
            error: None,
        },
        RecordingState::Error(msg) => RecordingStateDto {
            state: "error".to_string(),
            duration_ms: None,
            error: Some(msg.clone()),
        },
    }
}

/// DTO for recording state
#[derive(serde::Serialize)]
pub struct RecordingStateDto {
    pub state: String,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

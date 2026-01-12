//! Shortcut Handler
//!
//! Register and handle global keyboard shortcuts.

use crate::audio::{AudioCapture, AudioConfig};
use crate::{AppState, RecordingState};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Register all global shortcuts
pub fn register_shortcuts(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let state = app.state::<AppState>();
    let config = state.config.read();

    // Parse shortcut from config
    let record_shortcut: Shortcut = config.shortcuts.record.parse()?;

    // Register the shortcut
    app.global_shortcut()
        .on_shortcut(record_shortcut, move |app, shortcut, event| {
            handle_record_shortcut(app, shortcut, event.state);
        })?;

    tracing::info!("Global shortcuts registered");
    Ok(())
}

/// Handle record shortcut event
fn handle_record_shortcut(app: &AppHandle, _shortcut: &Shortcut, event: ShortcutState) {
    let state = app.state::<AppState>();
    let config = state.config.read();

    match config.recording.mode {
        crate::config::RecordingMode::PushToTalk => {
            handle_push_to_talk(app, event);
        }
        crate::config::RecordingMode::Toggle => {
            handle_toggle(app, event);
        }
    }
}

/// Handle push-to-talk mode
fn handle_push_to_talk(app: &AppHandle, event: ShortcutState) {
    let app_clone = app.clone();

    match event {
        ShortcutState::Pressed => {
            tracing::debug!("PTT: Key pressed, starting recording");
            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_recording_internal(&app_clone).await {
                    tracing::error!("Failed to start recording: {}", e);
                }
            });
        }
        ShortcutState::Released => {
            tracing::debug!("PTT: Key released, stopping recording");
            tauri::async_runtime::spawn(async move {
                if let Err(e) = stop_recording_internal(&app_clone).await {
                    tracing::error!("Failed to stop recording: {}", e);
                }
            });
        }
    }
}

/// Handle toggle mode
fn handle_toggle(app: &AppHandle, event: ShortcutState) {
    if event != ShortcutState::Pressed {
        return;
    }

    let state = app.state::<AppState>();
    let should_start = {
        let recording_state = state.recording_state.read();
        match &*recording_state {
            RecordingState::Idle | RecordingState::Error(_) => Some(true),
            RecordingState::Recording { .. } => Some(false),
            RecordingState::Processing => None,
        }
    };

    let app_clone = app.clone();

    match should_start {
        Some(true) => {
            tracing::debug!("Toggle: Starting recording");
            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_recording_internal(&app_clone).await {
                    tracing::error!("Failed to start recording: {}", e);
                }
            });
        }
        Some(false) => {
            tracing::debug!("Toggle: Stopping recording");
            tauri::async_runtime::spawn(async move {
                if let Err(e) = stop_recording_internal(&app_clone).await {
                    tracing::error!("Failed to stop recording: {}", e);
                }
            });
        }
        None => {
            tracing::debug!("Toggle: Ignored, currently processing");
        }
    }
}

/// Unregister all shortcuts
pub fn unregister_shortcuts(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    app.global_shortcut().unregister_all()?;
    tracing::info!("Global shortcuts unregistered");
    Ok(())
}

/// Re-register shortcuts after config change
pub fn update_shortcuts(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    unregister_shortcuts(app)?;

    let state = app.state::<AppState>();
    let config = state.config.read();

    let record_shortcut: Shortcut = config.shortcuts.record.parse()?;

    let app_clone = app.clone();
    app.global_shortcut()
        .on_shortcut(record_shortcut, move |_app, shortcut, event| {
            handle_record_shortcut(&app_clone, shortcut, event.state);
        })?;

    tracing::info!("Global shortcuts updated");
    Ok(())
}

/// Internal function to start recording
async fn start_recording_internal(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

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

    // Get max_duration from config
    let max_duration = {
        let config = state.config.read();
        config.recording.max_duration
    };

    // Initialize audio capture with appropriate buffer size
    let audio_config = AudioConfig {
        buffer_duration_ms: max_duration * 1000,
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

    // Show recording indicator
    show_recording_indicator(app);

    // Emit event
    let _ = app.emit("recording:state-changed", "recording");

    tracing::info!("Recording started via shortcut");
    Ok(())
}

/// Internal function to stop recording and transcribe
async fn stop_recording_internal(app: &AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();

    tracing::info!("Stopping recording via shortcut");

    // Switch indicator to processing state
    show_processing_indicator(app);

    // Get audio samples
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
                hide_recording_indicator(app);
                return Err("Not recording".to_string());
            }
        }
    };

    // Check duration
    let duration = {
        let recording_state = state.recording_state.read();
        match &*recording_state {
            RecordingState::Recording { started_at } => started_at.elapsed(),
            _ => {
                hide_recording_indicator(app);
                return Err("Not recording".to_string());
            }
        }
    };

    tracing::info!(
        "Recording stopped: {} samples, {:.2}s duration",
        raw_samples.len(),
        duration.as_secs_f32()
    );

    // Update state to processing
    *state.recording_state.write() = RecordingState::Processing;
    let _ = app.emit("recording:state-changed", "processing");

    // Check for minimum audio
    if raw_samples.len() < 1600 {
        *state.recording_state.write() = RecordingState::Idle;
        let _ = app.emit("recording:state-changed", "idle");
        hide_recording_indicator(app);
        return Err("Recording too short".to_string());
    }

    // Use transcription service
    let service = state.transcription_service.clone();
    let result = service
        .process_recording(app, raw_samples, device_sample_rate)
        .await;

    // Update state based on result
    match &result {
        Ok(_) => {
            *state.recording_state.write() = RecordingState::Idle;
            let _ = app.emit("recording:state-changed", "idle");
        }
        Err(e) => {
            *state.recording_state.write() = RecordingState::Error(e.clone());
            let _ = app.emit("recording:state-changed", "error");
        }
    }

    // Hide indicator
    hide_recording_indicator(app);

    result
}

/// Show the recording indicator overlay window
fn show_recording_indicator(app: &AppHandle) {
    let state = app.state::<AppState>();
    let show_indicator = {
        let config = state.config.read();
        config.ui.show_indicator
    };

    if !show_indicator {
        tracing::debug!("Recording indicator disabled in settings");
        return;
    }

    if let Some(window) = app.get_webview_window("recording-indicator") {
        let _ = window.show();

        let window_clone = window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = window_clone.emit("recording:state-changed", "recording");
        });

        tracing::debug!("Recording indicator shown");
    } else {
        tracing::warn!("Recording indicator window not found");
    }
}

/// Switch indicator to processing state
fn show_processing_indicator(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("recording-indicator") {
        let _ = window.emit("indicator:processing", ());
        let _ = window.emit("recording:state-changed", "processing");
        tracing::debug!("Recording indicator switched to processing");
    }

    let _ = app.emit("recording:state-changed", "processing");
}

/// Hide the recording indicator overlay window
fn hide_recording_indicator(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("recording-indicator") {
        let _ = window.hide();
        tracing::debug!("Recording indicator hidden");
    }
}

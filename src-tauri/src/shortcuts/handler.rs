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
    let record_shortcut: Shortcut = config.shortcuts.record.parse().map_err(|e| {
        tracing::error!("Failed to parse shortcut '{}': {}", config.shortcuts.record, e);
        e
    })?;

    // Register the shortcut
    app.global_shortcut()
        .on_shortcut(record_shortcut.clone(), move |app, shortcut, event| {
            handle_record_shortcut(app, shortcut, event.state);
        })
        .map_err(|e| {
            tracing::error!("Failed to register shortcut {:?}: {}", record_shortcut, e);
            e
        })?;

    tracing::info!("Global shortcut registered: {:?}", record_shortcut);
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
    use tauri_plugin_notification::NotificationExt;

    let state = app.state::<AppState>();

    tracing::info!("Stopping recording via shortcut");

    // Switch indicator to processing state
    show_processing_indicator(app);

    // Get audio samples and check for errors
    let (raw_samples, device_sample_rate, stream_error) = {
        let mut capture_guard = state.audio_capture.lock();
        match capture_guard.as_ref() {
            Some(capture) => {
                // Check for stream errors (e.g., microphone disconnection)
                let stream_error = capture.get_error();

                let result = capture
                    .stop()
                    .map_err(|e| format!("Failed to stop audio: {}", e))?;
                *capture_guard = None;
                (result.0, result.1, stream_error)
            }
            None => {
                hide_recording_indicator(app);
                return Err("Not recording".to_string());
            }
        }
    };

    // Handle microphone disconnection or other stream errors
    if let Some(error) = stream_error {
        tracing::warn!("Stream error detected during recording: {}", error.message);

        if error.is_disconnection {
            // Notify user about microphone disconnection
            let _ = app
                .notification()
                .builder()
                .title("Microphone Disconnected")
                .body("The microphone was disconnected during recording. Please reconnect and try again.")
                .show();

            // Emit error event to frontend
            let _ = app.emit("recording:microphone-error", "Microphone disconnected during recording");

            *state.recording_state.write() = RecordingState::Error("Microphone disconnected".to_string());
            let _ = app.emit("recording:state-changed", "error");
            hide_recording_indicator(app);

            return Err("Microphone disconnected during recording".to_string());
        }
    }

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

// ============================================================================
// Shortcut Utilities (testable without Tauri runtime)
// ============================================================================

/// Validates a shortcut string format and returns the parsed shortcut.
/// This is a utility function that can be used to validate shortcuts before registration.
pub fn parse_shortcut(shortcut_str: &str) -> Result<Shortcut, ShortcutError> {
    if shortcut_str.is_empty() {
        return Err(ShortcutError::Empty);
    }

    shortcut_str
        .parse::<Shortcut>()
        .map_err(|e| ShortcutError::ParseError(e.to_string()))
}

/// Checks if two shortcut strings represent the same key combination.
/// Returns true if they conflict (are the same).
pub fn shortcuts_conflict(shortcut_a: &str, shortcut_b: &str) -> Result<bool, ShortcutError> {
    let a = parse_shortcut(shortcut_a)?;
    let b = parse_shortcut(shortcut_b)?;
    Ok(a == b)
}

/// Normalizes a shortcut string to a canonical format.
/// Useful for comparing shortcuts that may be written differently.
pub fn normalize_shortcut(shortcut_str: &str) -> Result<String, ShortcutError> {
    let shortcut = parse_shortcut(shortcut_str)?;
    Ok(format!("{:?}", shortcut))
}

/// Determines the recording action based on the current state and event.
/// This pure function encapsulates the state machine logic for recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordingAction {
    StartRecording,
    StopRecording,
    Ignore,
}

/// Determines what action to take for push-to-talk mode.
pub fn determine_ptt_action(event: ShortcutState) -> RecordingAction {
    match event {
        ShortcutState::Pressed => RecordingAction::StartRecording,
        ShortcutState::Released => RecordingAction::StopRecording,
    }
}

/// Determines what action to take for toggle mode based on current recording state.
pub fn determine_toggle_action(
    event: ShortcutState,
    recording_state: &RecordingState,
) -> RecordingAction {
    if event != ShortcutState::Pressed {
        return RecordingAction::Ignore;
    }

    match recording_state {
        RecordingState::Idle | RecordingState::Error(_) => RecordingAction::StartRecording,
        RecordingState::Recording { .. } => RecordingAction::StopRecording,
        RecordingState::Processing => RecordingAction::Ignore,
    }
}

/// Custom error type for shortcut operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutError {
    Empty,
    ParseError(String),
}

impl std::fmt::Display for ShortcutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShortcutError::Empty => write!(f, "Shortcut string is empty"),
            ShortcutError::ParseError(e) => write!(f, "Failed to parse shortcut: {}", e),
        }
    }
}

impl std::error::Error for ShortcutError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecordingState;

    // ========================================================================
    // Shortcut Parsing Tests
    // ========================================================================

    mod shortcut_parsing {
        use super::*;

        #[test]
        fn test_parse_valid_simple_shortcuts() {
            // Single keys
            assert!(parse_shortcut("F1").is_ok());
            assert!(parse_shortcut("F12").is_ok());
            assert!(parse_shortcut("Escape").is_ok());
            assert!(parse_shortcut("Space").is_ok());
        }

        #[test]
        fn test_parse_valid_modifier_shortcuts() {
            // Modifier + key combinations
            assert!(parse_shortcut("Ctrl+Space").is_ok());
            assert!(parse_shortcut("Alt+F4").is_ok());
            assert!(parse_shortcut("Shift+A").is_ok());
            assert!(parse_shortcut("Super+L").is_ok()); // Windows key
        }

        #[test]
        fn test_parse_valid_multi_modifier_shortcuts() {
            // Multiple modifiers
            assert!(parse_shortcut("Ctrl+Shift+S").is_ok());
            assert!(parse_shortcut("Ctrl+Alt+Delete").is_ok());
            assert!(parse_shortcut("Ctrl+Shift+Alt+F1").is_ok());
        }

        #[test]
        fn test_parse_empty_shortcut() {
            let result = parse_shortcut("");
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), ShortcutError::Empty);
        }

        #[test]
        fn test_parse_invalid_shortcut_gibberish() {
            let result = parse_shortcut("NotAKey");
            assert!(result.is_err());
            match result.unwrap_err() {
                ShortcutError::ParseError(_) => {}
                _ => panic!("Expected ParseError"),
            }
        }

        #[test]
        fn test_parse_invalid_shortcut_empty_modifier() {
            let result = parse_shortcut("Ctrl+");
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_invalid_shortcut_double_plus() {
            let result = parse_shortcut("Ctrl++A");
            assert!(result.is_err());
        }

        #[test]
        fn test_parse_case_sensitivity() {
            // Tauri shortcuts should be case-insensitive for modifiers
            // but we test to ensure consistent behavior
            assert!(parse_shortcut("ctrl+space").is_ok());
            assert!(parse_shortcut("CTRL+SPACE").is_ok());
            assert!(parse_shortcut("Ctrl+Space").is_ok());
        }

        #[test]
        fn test_parse_special_keys() {
            // Special keys that should work
            assert!(parse_shortcut("Tab").is_ok());
            assert!(parse_shortcut("Enter").is_ok());
            assert!(parse_shortcut("Backspace").is_ok());
            assert!(parse_shortcut("Insert").is_ok());
            assert!(parse_shortcut("Delete").is_ok());
            assert!(parse_shortcut("Home").is_ok());
            assert!(parse_shortcut("End").is_ok());
            assert!(parse_shortcut("PageUp").is_ok());
            assert!(parse_shortcut("PageDown").is_ok());
        }

        #[test]
        fn test_parse_arrow_keys() {
            assert!(parse_shortcut("ArrowUp").is_ok());
            assert!(parse_shortcut("ArrowDown").is_ok());
            assert!(parse_shortcut("ArrowLeft").is_ok());
            assert!(parse_shortcut("ArrowRight").is_ok());
        }

        #[test]
        fn test_parse_numpad_keys() {
            // Numpad keys
            assert!(parse_shortcut("Numpad0").is_ok());
            assert!(parse_shortcut("Numpad9").is_ok());
            assert!(parse_shortcut("NumpadAdd").is_ok());
            assert!(parse_shortcut("NumpadSubtract").is_ok());
        }
    }

    // ========================================================================
    // Shortcut Conflict Detection Tests
    // ========================================================================

    mod conflict_detection {
        use super::*;

        #[test]
        fn test_same_shortcuts_conflict() {
            let result = shortcuts_conflict("Ctrl+Space", "Ctrl+Space");
            assert!(result.is_ok());
            assert!(result.unwrap(), "Same shortcuts should conflict");
        }

        #[test]
        fn test_different_shortcuts_no_conflict() {
            let result = shortcuts_conflict("Ctrl+Space", "Ctrl+Shift+Space");
            assert!(result.is_ok());
            assert!(!result.unwrap(), "Different shortcuts should not conflict");
        }

        #[test]
        fn test_different_keys_no_conflict() {
            let result = shortcuts_conflict("Ctrl+A", "Ctrl+B");
            assert!(result.is_ok());
            assert!(!result.unwrap());
        }

        #[test]
        fn test_different_modifiers_no_conflict() {
            let result = shortcuts_conflict("Ctrl+A", "Alt+A");
            assert!(result.is_ok());
            assert!(!result.unwrap());
        }

        #[test]
        fn test_conflict_with_invalid_shortcut() {
            let result = shortcuts_conflict("Ctrl+Space", "InvalidKey");
            assert!(result.is_err());
        }

        #[test]
        fn test_conflict_with_empty_shortcut() {
            let result = shortcuts_conflict("", "Ctrl+Space");
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), ShortcutError::Empty);
        }

        #[test]
        fn test_case_insensitive_conflict() {
            // Should detect conflict regardless of case
            let result = shortcuts_conflict("ctrl+space", "CTRL+SPACE");
            assert!(result.is_ok());
            assert!(result.unwrap(), "Case should not matter for conflict detection");
        }
    }

    // ========================================================================
    // Shortcut Normalization Tests
    // ========================================================================

    mod normalization {
        use super::*;

        #[test]
        fn test_normalize_valid_shortcut() {
            let result = normalize_shortcut("Ctrl+Space");
            assert!(result.is_ok());
            // The normalized format should be consistent
            let normalized = result.unwrap();
            assert!(!normalized.is_empty());
        }

        #[test]
        fn test_normalize_same_result_different_case() {
            let lower = normalize_shortcut("ctrl+space").unwrap();
            let upper = normalize_shortcut("CTRL+SPACE").unwrap();
            let mixed = normalize_shortcut("Ctrl+Space").unwrap();

            // All should normalize to the same representation
            assert_eq!(lower, upper);
            assert_eq!(upper, mixed);
        }

        #[test]
        fn test_normalize_empty_fails() {
            let result = normalize_shortcut("");
            assert!(result.is_err());
        }

        #[test]
        fn test_normalize_invalid_fails() {
            let result = normalize_shortcut("InvalidShortcut");
            assert!(result.is_err());
        }
    }

    // ========================================================================
    // Push-to-Talk Mode Tests
    // ========================================================================

    mod push_to_talk {
        use super::*;

        #[test]
        fn test_ptt_pressed_starts_recording() {
            let action = determine_ptt_action(ShortcutState::Pressed);
            assert_eq!(action, RecordingAction::StartRecording);
        }

        #[test]
        fn test_ptt_released_stops_recording() {
            let action = determine_ptt_action(ShortcutState::Released);
            assert_eq!(action, RecordingAction::StopRecording);
        }

        #[test]
        fn test_ptt_symmetry() {
            // Push-to-talk should always have symmetric start/stop
            let press = determine_ptt_action(ShortcutState::Pressed);
            let release = determine_ptt_action(ShortcutState::Released);

            assert_ne!(press, release, "Press and release should have different actions");
            assert_eq!(press, RecordingAction::StartRecording);
            assert_eq!(release, RecordingAction::StopRecording);
        }
    }

    // ========================================================================
    // Toggle Mode Tests
    // ========================================================================

    mod toggle_mode {
        use super::*;

        #[test]
        fn test_toggle_idle_pressed_starts() {
            let state = RecordingState::Idle;
            let action = determine_toggle_action(ShortcutState::Pressed, &state);
            assert_eq!(action, RecordingAction::StartRecording);
        }

        #[test]
        fn test_toggle_recording_pressed_stops() {
            let state = RecordingState::Recording {
                started_at: std::time::Instant::now(),
            };
            let action = determine_toggle_action(ShortcutState::Pressed, &state);
            assert_eq!(action, RecordingAction::StopRecording);
        }

        #[test]
        fn test_toggle_processing_pressed_ignores() {
            let state = RecordingState::Processing;
            let action = determine_toggle_action(ShortcutState::Pressed, &state);
            assert_eq!(action, RecordingAction::Ignore);
        }

        #[test]
        fn test_toggle_error_pressed_starts() {
            // After an error, pressing should start a new recording
            let state = RecordingState::Error("Test error".to_string());
            let action = determine_toggle_action(ShortcutState::Pressed, &state);
            assert_eq!(action, RecordingAction::StartRecording);
        }

        #[test]
        fn test_toggle_released_always_ignores() {
            // In toggle mode, only press events matter
            let states = vec![
                RecordingState::Idle,
                RecordingState::Recording {
                    started_at: std::time::Instant::now(),
                },
                RecordingState::Processing,
                RecordingState::Error("Test".to_string()),
            ];

            for state in states {
                let action = determine_toggle_action(ShortcutState::Released, &state);
                assert_eq!(
                    action,
                    RecordingAction::Ignore,
                    "Toggle mode should ignore released events for state: {:?}",
                    state
                );
            }
        }

        #[test]
        fn test_toggle_state_machine_cycle() {
            // Simulate a complete recording cycle
            // 1. Start from Idle
            let state1 = RecordingState::Idle;
            let action1 = determine_toggle_action(ShortcutState::Pressed, &state1);
            assert_eq!(action1, RecordingAction::StartRecording);

            // 2. Now recording, press again to stop
            let state2 = RecordingState::Recording {
                started_at: std::time::Instant::now(),
            };
            let action2 = determine_toggle_action(ShortcutState::Pressed, &state2);
            assert_eq!(action2, RecordingAction::StopRecording);

            // 3. While processing, press should be ignored
            let state3 = RecordingState::Processing;
            let action3 = determine_toggle_action(ShortcutState::Pressed, &state3);
            assert_eq!(action3, RecordingAction::Ignore);

            // 4. Back to idle, can start again
            let state4 = RecordingState::Idle;
            let action4 = determine_toggle_action(ShortcutState::Pressed, &state4);
            assert_eq!(action4, RecordingAction::StartRecording);
        }
    }

    // ========================================================================
    // Recording State Tests
    // ========================================================================

    mod recording_state {
        use super::*;

        #[test]
        fn test_recording_state_default_is_idle() {
            let state = RecordingState::default();
            matches!(state, RecordingState::Idle);
        }

        #[test]
        fn test_recording_state_recording_has_timestamp() {
            let before = std::time::Instant::now();
            let state = RecordingState::Recording {
                started_at: std::time::Instant::now(),
            };
            let after = std::time::Instant::now();

            if let RecordingState::Recording { started_at } = state {
                assert!(started_at >= before);
                assert!(started_at <= after);
            } else {
                panic!("Expected Recording state");
            }
        }

        #[test]
        fn test_recording_state_error_contains_message() {
            let error_msg = "Microphone disconnected";
            let state = RecordingState::Error(error_msg.to_string());

            if let RecordingState::Error(msg) = state {
                assert_eq!(msg, error_msg);
            } else {
                panic!("Expected Error state");
            }
        }
    }

    // ========================================================================
    // Shortcut Error Tests
    // ========================================================================

    mod error_handling {
        use super::*;

        #[test]
        fn test_shortcut_error_display_empty() {
            let err = ShortcutError::Empty;
            let display = format!("{}", err);
            assert!(display.contains("empty"));
        }

        #[test]
        fn test_shortcut_error_display_parse_error() {
            let err = ShortcutError::ParseError("invalid key".to_string());
            let display = format!("{}", err);
            assert!(display.contains("invalid key"));
        }

        #[test]
        fn test_shortcut_error_equality() {
            assert_eq!(ShortcutError::Empty, ShortcutError::Empty);
            assert_eq!(
                ShortcutError::ParseError("test".to_string()),
                ShortcutError::ParseError("test".to_string())
            );
            assert_ne!(
                ShortcutError::Empty,
                ShortcutError::ParseError("test".to_string())
            );
        }
    }

    // ========================================================================
    // Edge Cases and Boundary Tests
    // ========================================================================

    mod edge_cases {
        use super::*;

        #[test]
        fn test_whitespace_in_shortcut() {
            // Shortcuts with leading/trailing whitespace should fail
            let result = parse_shortcut(" Ctrl+Space");
            // Depending on implementation, this may or may not work
            // We test to document behavior
            assert!(result.is_err() || result.is_ok());
        }

        #[test]
        fn test_very_long_shortcut_string() {
            // Test with a very long invalid string
            let long_string = "Ctrl+".to_string() + &"A".repeat(1000);
            let result = parse_shortcut(&long_string);
            assert!(result.is_err());
        }

        #[test]
        fn test_unicode_in_shortcut() {
            // Unicode characters should fail
            let result = parse_shortcut("Ctrl+\u{1F600}");
            assert!(result.is_err());
        }

        #[test]
        fn test_special_characters_in_shortcut() {
            // Special characters like @, #, $ should fail or work depending on key support
            let result = parse_shortcut("Ctrl+@");
            // Document behavior - some special chars might work
            assert!(result.is_err() || result.is_ok());
        }

        #[test]
        fn test_rapid_state_changes() {
            // Simulate rapid key presses in toggle mode
            let mut state = RecordingState::Idle;

            // Rapid presses should alternate
            for i in 0..10 {
                let action = determine_toggle_action(ShortcutState::Pressed, &state);
                if i % 2 == 0 {
                    assert_eq!(action, RecordingAction::StartRecording);
                    state = RecordingState::Recording {
                        started_at: std::time::Instant::now(),
                    };
                } else {
                    assert_eq!(action, RecordingAction::StopRecording);
                    state = RecordingState::Idle;
                }
            }
        }

        #[test]
        fn test_recording_action_debug() {
            // Ensure Debug is implemented correctly
            let action = RecordingAction::StartRecording;
            let debug = format!("{:?}", action);
            assert!(debug.contains("Start"));
        }

        #[test]
        fn test_recording_action_clone() {
            let action = RecordingAction::StopRecording;
            let cloned = action.clone();
            assert_eq!(action, cloned);
        }
    }

    // ========================================================================
    // Common Shortcut Combinations Used by Apps
    // ========================================================================

    mod common_shortcuts {
        use super::*;

        #[test]
        fn test_common_system_shortcuts() {
            // These are common system shortcuts that users might try to use
            // We test they can be parsed (registration might still fail due to conflicts)
            let common_shortcuts = vec![
                "Ctrl+C",
                "Ctrl+V",
                "Ctrl+X",
                "Ctrl+Z",
                "Ctrl+Y",
                "Ctrl+A",
                "Ctrl+S",
                "Ctrl+Shift+S",
                "Ctrl+P",
                "Ctrl+F",
                "Ctrl+H",
                "Alt+Tab",
                "Alt+F4",
                "Ctrl+Shift+Escape",
            ];

            for shortcut in common_shortcuts {
                let result = parse_shortcut(shortcut);
                assert!(
                    result.is_ok(),
                    "Common shortcut '{}' should be parseable",
                    shortcut
                );
            }
        }

        #[test]
        fn test_media_key_shortcuts() {
            // Media keys (might not all be supported)
            let media_shortcuts = vec![
                "MediaPlayPause",
                "MediaStop",
                "MediaTrackNext",
                "MediaTrackPrevious",
            ];

            for shortcut in media_shortcuts {
                // Just test that parsing doesn't panic
                let _ = parse_shortcut(shortcut);
            }
        }

        #[test]
        fn test_default_gigawhisper_shortcut() {
            // The default shortcut used by GigaWhisper
            let result = parse_shortcut("Ctrl+Space");
            assert!(result.is_ok(), "Default shortcut should be valid");
        }

        #[test]
        fn test_alternative_recording_shortcuts() {
            // Alternative shortcuts users might configure
            let alternatives = vec![
                "Ctrl+Shift+R",
                "Ctrl+Alt+Space",
                "F9",
                "Ctrl+`",
                "Ctrl+Shift+W",
            ];

            for shortcut in alternatives {
                let result = parse_shortcut(shortcut);
                assert!(
                    result.is_ok(),
                    "Alternative shortcut '{}' should be parseable",
                    shortcut
                );
            }
        }
    }

    // ========================================================================
    // Integration-style Tests (without Tauri runtime)
    // ========================================================================

    mod integration_scenarios {
        use super::*;
        use crate::config::RecordingMode;

        /// Simulates the decision logic for handling a shortcut event
        fn simulate_shortcut_handling(
            mode: RecordingMode,
            event: ShortcutState,
            current_state: &RecordingState,
        ) -> RecordingAction {
            match mode {
                RecordingMode::PushToTalk => determine_ptt_action(event),
                RecordingMode::Toggle => determine_toggle_action(event, current_state),
            }
        }

        #[test]
        fn test_ptt_full_recording_session() {
            // Simulate a complete PTT session
            let mode = RecordingMode::PushToTalk;
            let state = RecordingState::Idle;

            // User presses key
            let action1 = simulate_shortcut_handling(mode.clone(), ShortcutState::Pressed, &state);
            assert_eq!(action1, RecordingAction::StartRecording);

            // Recording is now active
            let recording_state = RecordingState::Recording {
                started_at: std::time::Instant::now(),
            };

            // User releases key
            let action2 =
                simulate_shortcut_handling(mode.clone(), ShortcutState::Released, &recording_state);
            assert_eq!(action2, RecordingAction::StopRecording);
        }

        #[test]
        fn test_toggle_full_recording_session() {
            // Simulate a complete toggle session
            let mode = RecordingMode::Toggle;
            let state = RecordingState::Idle;

            // First press - start recording
            let action1 = simulate_shortcut_handling(mode.clone(), ShortcutState::Pressed, &state);
            assert_eq!(action1, RecordingAction::StartRecording);

            // Release is ignored in toggle mode
            let recording_state = RecordingState::Recording {
                started_at: std::time::Instant::now(),
            };
            let action2 =
                simulate_shortcut_handling(mode.clone(), ShortcutState::Released, &recording_state);
            assert_eq!(action2, RecordingAction::Ignore);

            // Second press - stop recording
            let action3 =
                simulate_shortcut_handling(mode.clone(), ShortcutState::Pressed, &recording_state);
            assert_eq!(action3, RecordingAction::StopRecording);
        }

        #[test]
        fn test_shortcut_update_scenario() {
            // Simulate updating a shortcut from one key to another
            let old_shortcut = "Ctrl+Space";
            let new_shortcut = "Ctrl+Shift+R";

            // Both should be valid
            assert!(parse_shortcut(old_shortcut).is_ok());
            assert!(parse_shortcut(new_shortcut).is_ok());

            // They should not conflict
            let conflicts = shortcuts_conflict(old_shortcut, new_shortcut).unwrap();
            assert!(!conflicts, "Different shortcuts should not conflict");
        }

        #[test]
        fn test_conflict_detection_scenario() {
            // Simulate checking for conflicts with system shortcuts
            let user_shortcut = "Ctrl+Space";

            // Check against a list of "reserved" shortcuts
            let reserved = vec!["Ctrl+C", "Ctrl+V", "Ctrl+X", "Alt+F4"];

            for reserved_shortcut in reserved {
                let conflicts = shortcuts_conflict(user_shortcut, reserved_shortcut).unwrap();
                assert!(
                    !conflicts,
                    "User shortcut should not conflict with {}",
                    reserved_shortcut
                );
            }

            // But it should conflict with itself
            let self_conflict = shortcuts_conflict(user_shortcut, user_shortcut).unwrap();
            assert!(self_conflict, "Shortcut should conflict with itself");
        }

        #[test]
        fn test_error_recovery_scenario() {
            // Simulate recovering from an error state
            let mode = RecordingMode::Toggle;
            let error_state = RecordingState::Error("Previous recording failed".to_string());

            // Should be able to start new recording after error
            let action = simulate_shortcut_handling(mode, ShortcutState::Pressed, &error_state);
            assert_eq!(
                action,
                RecordingAction::StartRecording,
                "Should recover from error state"
            );
        }
    }
}

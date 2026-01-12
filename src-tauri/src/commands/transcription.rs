//! Transcription Commands
//!
//! Handle transcription status and results.

use crate::transcription::TranscriptionStatus;
use crate::AppState;
use tauri::State;

/// Get transcription status
#[tauri::command]
pub fn get_transcription_status(state: State<'_, AppState>) -> TranscriptionStatus {
    // Update status from current config before returning
    let config = state.config.read();
    state.transcription_service.update_status_from_config(&config);
    drop(config);

    state.transcription_service.get_status()
}

/// Preload the transcription model (for faster first transcription)
#[tauri::command]
pub async fn preload_model(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.read().clone();
    state.transcription_service.preload_model(&config)
}

/// Unload the transcription model to free memory
#[tauri::command]
pub fn unload_model(state: State<'_, AppState>) {
    state.transcription_service.unload_model();
}

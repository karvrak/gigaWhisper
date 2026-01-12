//! Model Commands
//!
//! Tauri commands for model management (list, download, delete).

use crate::config::{models_dir, WhisperModel};
use crate::models::{self, DownloadProgress, ModelInfo};
use tauri::{AppHandle, Emitter};

/// List all models with download status
#[tauri::command]
pub fn list_models() -> Vec<ModelInfo> {
    models::list_models()
}

/// Check if a specific model is downloaded
#[tauri::command]
pub fn is_model_downloaded(model: String) -> Result<bool, String> {
    let whisper_model = parse_model(&model)?;
    Ok(models::is_model_downloaded(&whisper_model))
}

/// Download a model with progress events
#[tauri::command]
pub async fn download_model(app: AppHandle, model: String) -> Result<String, String> {
    let whisper_model = parse_model(&model)?;
    let dest_dir = models_dir();

    tracing::info!("Starting download for model: {}", model);

    // Create progress callback that emits events to frontend
    let app_clone = app.clone();
    let model_clone = model.clone();
    let progress_callback = Box::new(move |progress: DownloadProgress| {
        let _ = app_clone.emit(
            "model-download-progress",
            serde_json::json!({
                "model": model_clone,
                "downloaded_bytes": progress.downloaded_bytes,
                "total_bytes": progress.total_bytes,
                "percentage": progress.percentage,
                "speed_bps": progress.speed_bps
            }),
        );
    });

    // Perform download
    match models::download_model(&whisper_model, dest_dir, Some(progress_callback)).await {
        Ok(path) => {
            let _ = app.emit(
                "model-download-complete",
                serde_json::json!({
                    "model": model,
                    "path": path.display().to_string()
                }),
            );
            Ok(path.display().to_string())
        }
        Err(e) => {
            let error_msg = e.to_string();
            let _ = app.emit(
                "model-download-error",
                serde_json::json!({
                    "model": model,
                    "error": error_msg
                }),
            );
            Err(error_msg)
        }
    }
}

/// Delete a downloaded model
#[tauri::command]
pub fn delete_model(model: String) -> Result<(), String> {
    let whisper_model = parse_model(&model)?;
    models::delete_model(&whisper_model).map_err(|e| e.to_string())
}

/// Cancel an ongoing model download
#[tauri::command]
pub fn cancel_model_download(app: AppHandle, model: String) -> Result<bool, String> {
    let whisper_model = parse_model(&model)?;
    let cancelled = models::cancel_download(&whisper_model);

    if cancelled {
        let _ = app.emit(
            "model-download-cancelled",
            serde_json::json!({
                "model": model
            }),
        );
    }

    Ok(cancelled)
}

/// Check if a model is currently being downloaded
#[tauri::command]
pub fn is_model_downloading(model: String) -> Result<bool, String> {
    let whisper_model = parse_model(&model)?;
    Ok(models::is_downloading(&whisper_model))
}

/// Get the recommended model based on system resources
#[tauri::command]
pub fn get_recommended_model() -> String {
    format!("{:?}", models::recommend_model()).to_lowercase()
}

/// Parse model string to WhisperModel enum
fn parse_model(model: &str) -> Result<WhisperModel, String> {
    match model.to_lowercase().as_str() {
        "tiny" => Ok(WhisperModel::Tiny),
        "base" => Ok(WhisperModel::Base),
        "small" => Ok(WhisperModel::Small),
        "medium" => Ok(WhisperModel::Medium),
        "large" => Ok(WhisperModel::Large),
        _ => Err(format!("Unknown model: {}", model)),
    }
}

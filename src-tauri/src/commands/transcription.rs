//! Transcription Commands
//!
//! Handle transcription status and results.

use crate::transcription::{TranscriptionStatus, WhisperProvider};
use crate::utils::{metrics, CpuInfo, MetricsSummary, TranscriptionRecord};
use crate::AppState;
use serde::Serialize;
use tauri::State;

/// GPU acceleration information
#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    /// Whether GPU acceleration is available in this build
    pub available: bool,
    /// Name of the GPU backend (CUDA, Vulkan, or None)
    pub backend: String,
    /// Whether GPU is currently enabled in settings
    pub enabled: bool,
}

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

/// Get GPU acceleration information
#[tauri::command]
pub fn get_gpu_info(state: State<'_, AppState>) -> GpuInfo {
    let config = state.config.read();
    GpuInfo {
        available: WhisperProvider::is_gpu_available(),
        backend: WhisperProvider::gpu_backend_name().to_string(),
        enabled: config.transcription.local.gpu_enabled,
    }
}

/// Get CPU information for performance optimization
#[tauri::command]
pub fn get_cpu_info() -> CpuInfo {
    CpuInfo::detect()
}

/// Get performance metrics summary
#[tauri::command]
pub fn get_metrics_summary() -> MetricsSummary {
    metrics().read().get_summary()
}

/// Get recent transcription records
#[tauri::command]
pub fn get_recent_metrics(count: Option<usize>) -> Vec<TranscriptionRecord> {
    metrics().read().get_recent(count.unwrap_or(10))
}

/// Reset performance metrics
#[tauri::command]
pub fn reset_metrics() {
    metrics().write().reset();
}

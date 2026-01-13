//! Model Manager
//!
//! Manage Whisper model files.

use crate::config::{models_dir, WhisperModel};
use std::path::PathBuf;

/// Model information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub model: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub downloaded: bool,
}

/// Get path for a specific model
pub fn model_path(model: &WhisperModel) -> PathBuf {
    models_dir().join(model.filename())
}

/// Check if a model is downloaded
pub fn is_model_downloaded(model: &WhisperModel) -> bool {
    model_path(model).exists()
}

/// List all models with their status
pub fn list_models() -> Vec<ModelInfo> {
    use WhisperModel::*;

    [Tiny, Base, Small, Medium, Large]
        .iter()
        .map(|model| {
            let path = model_path(model);
            let downloaded = path.exists();
            let size_bytes = if downloaded {
                std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            } else {
                model.size_bytes()
            };

            ModelInfo {
                model: format!("{:?}", model).to_lowercase(),
                path,
                size_bytes,
                downloaded,
            }
        })
        .collect()
}

/// Delete a downloaded model
pub fn delete_model(model: &WhisperModel) -> Result<(), std::io::Error> {
    let path = model_path(model);
    if path.exists() {
        std::fs::remove_file(&path)?;
        tracing::info!("Deleted model: {:?}", path);
    }
    Ok(())
}

/// Get recommended model based on available RAM
pub fn recommend_model() -> WhisperModel {
    // Get system memory using Windows API
    let total_ram_gb = get_system_memory_gb().unwrap_or(8);

    if total_ram_gb >= 16 {
        WhisperModel::Medium
    } else if total_ram_gb >= 8 {
        WhisperModel::Small
    } else if total_ram_gb >= 4 {
        WhisperModel::Base
    } else {
        WhisperModel::Tiny
    }
}

/// Get total system memory in GB using Windows API
#[cfg(windows)]
fn get_system_memory_gb() -> Option<u64> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    unsafe {
        let mut mem_info = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };

        if GlobalMemoryStatusEx(&mut mem_info).is_ok() {
            // Convert bytes to GB
            Some(mem_info.ullTotalPhys / (1024 * 1024 * 1024))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn get_system_memory_gb() -> Option<u64> {
    // Fallback for non-Windows platforms
    Some(8)
}

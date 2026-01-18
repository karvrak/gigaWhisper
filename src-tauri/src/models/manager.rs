//! Model Manager
//!
//! Manage Whisper model files including quantized variants.

use crate::config::{models_dir, ModelQuantization, WhisperModel};
use std::path::PathBuf;

/// Model information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub model: String,
    pub quantization: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub downloaded: bool,
    pub is_quantized: bool,
}

/// Get path for a specific model with quantization
pub fn model_path_with_quantization(model: &WhisperModel, quant: &ModelQuantization) -> PathBuf {
    models_dir().join(model.filename_with_quantization(quant))
}

/// Get path for a specific model (F16 by default)
pub fn model_path(model: &WhisperModel) -> PathBuf {
    model_path_with_quantization(model, &ModelQuantization::F16)
}

/// Check if a model with specific quantization is downloaded
pub fn is_model_downloaded_with_quantization(model: &WhisperModel, quant: &ModelQuantization) -> bool {
    model_path_with_quantization(model, quant).exists()
}

/// Check if a model is downloaded (F16 by default)
pub fn is_model_downloaded(model: &WhisperModel) -> bool {
    is_model_downloaded_with_quantization(model, &ModelQuantization::F16)
}

/// List all models with their status (F16 only for backward compatibility)
pub fn list_models() -> Vec<ModelInfo> {
    list_models_with_quantization(&ModelQuantization::F16)
}

/// List all models with a specific quantization
pub fn list_models_with_quantization(quant: &ModelQuantization) -> Vec<ModelInfo> {
    WhisperModel::all()
        .iter()
        .map(|model| {
            let path = model_path_with_quantization(model, quant);
            let downloaded = path.exists();
            let size_bytes = if downloaded {
                std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            } else {
                model.size_bytes_with_quantization(quant)
            };

            ModelInfo {
                model: format!("{:?}", model).to_lowercase(),
                quantization: format!("{:?}", quant).to_lowercase(),
                path,
                size_bytes,
                downloaded,
                is_quantized: *quant != ModelQuantization::F16,
            }
        })
        .collect()
}

/// List all available model variants (all sizes x all quantizations)
pub fn list_all_model_variants() -> Vec<ModelInfo> {
    let mut all_variants = Vec::new();

    for model in WhisperModel::all() {
        for quant in ModelQuantization::all() {
            let path = model_path_with_quantization(model, quant);
            let downloaded = path.exists();
            let size_bytes = if downloaded {
                std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            } else {
                model.size_bytes_with_quantization(quant)
            };

            all_variants.push(ModelInfo {
                model: format!("{:?}", model).to_lowercase(),
                quantization: format!("{:?}", quant).to_lowercase(),
                path,
                size_bytes,
                downloaded,
                is_quantized: *quant != ModelQuantization::F16,
            });
        }
    }

    all_variants
}

/// Get all downloaded model variants
pub fn list_downloaded_models() -> Vec<ModelInfo> {
    list_all_model_variants()
        .into_iter()
        .filter(|m| m.downloaded)
        .collect()
}

/// Delete a downloaded model with specific quantization
pub fn delete_model_with_quantization(model: &WhisperModel, quant: &ModelQuantization) -> Result<(), std::io::Error> {
    let path = model_path_with_quantization(model, quant);
    if path.exists() {
        std::fs::remove_file(&path)?;
        tracing::info!("Deleted model: {:?}", path);
    }
    Ok(())
}

/// Delete a downloaded model (F16 by default)
pub fn delete_model(model: &WhisperModel) -> Result<(), std::io::Error> {
    delete_model_with_quantization(model, &ModelQuantization::F16)
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

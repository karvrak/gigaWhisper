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

    // SAFETY: GlobalMemoryStatusEx is safe to call because:
    // - MEMORYSTATUSEX is properly initialized with correct dwLength
    // - The structure is stack-allocated with appropriate size and alignment
    // - dwLength must be set before the call (Windows API requirement)
    // - We check the return value and return None on failure
    // - No resources or handles require cleanup after the call
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ModelInfo Tests
    // =========================================================================

    #[test]
    fn test_model_info_serialization() {
        let info = ModelInfo {
            model: "tiny".to_string(),
            quantization: "f16".to_string(),
            path: std::path::PathBuf::from("/models/ggml-tiny.bin"),
            size_bytes: 75_000_000,
            downloaded: true,
            is_quantized: false,
        };

        let json = serde_json::to_string(&info).expect("Failed to serialize");
        assert!(json.contains("tiny"));
        assert!(json.contains("f16"));
        assert!(json.contains("75000000"));
    }

    #[test]
    fn test_model_info_clone() {
        let info = ModelInfo {
            model: "base".to_string(),
            quantization: "q8_0".to_string(),
            path: std::path::PathBuf::from("/models/ggml-base-q8_0.bin"),
            size_bytes: 100_000_000,
            downloaded: false,
            is_quantized: true,
        };

        let cloned = info.clone();
        assert_eq!(info.model, cloned.model);
        assert_eq!(info.quantization, cloned.quantization);
        assert_eq!(info.size_bytes, cloned.size_bytes);
        assert_eq!(info.downloaded, cloned.downloaded);
        assert_eq!(info.is_quantized, cloned.is_quantized);
    }

    // =========================================================================
    // Model Path Tests
    // =========================================================================

    #[test]
    fn test_model_path_with_quantization_f16() {
        let path = model_path_with_quantization(&WhisperModel::Tiny, &ModelQuantization::F16);
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("ggml-tiny.bin"));
    }

    #[test]
    fn test_model_path_with_quantization_q8() {
        let path = model_path_with_quantization(&WhisperModel::Base, &ModelQuantization::Q8_0);
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("ggml-base-q8_0.bin"));
    }

    #[test]
    fn test_model_path_default() {
        let path = model_path(&WhisperModel::Small);
        let path_str = path.to_string_lossy();
        // Default is F16
        assert!(path_str.contains("ggml-small.bin"));
    }

    // =========================================================================
    // Model Downloaded Check Tests
    // =========================================================================

    #[test]
    fn test_is_model_downloaded_nonexistent() {
        // Model files shouldn't exist in test environment (unless pre-downloaded)
        // This test just verifies the function runs without error
        let _ = is_model_downloaded(&WhisperModel::Large);
    }

    #[test]
    fn test_is_model_downloaded_with_quantization() {
        // Verify function works with different quantizations
        let _ = is_model_downloaded_with_quantization(&WhisperModel::Tiny, &ModelQuantization::Q5_1);
    }

    // =========================================================================
    // List Models Tests
    // =========================================================================

    #[test]
    fn test_list_models_returns_all_sizes() {
        let models = list_models();

        // Should have all model sizes (Tiny, Base, Small, Medium, Large)
        assert!(!models.is_empty());

        let model_names: Vec<&str> = models.iter().map(|m| m.model.as_str()).collect();
        assert!(model_names.contains(&"tiny"));
        assert!(model_names.contains(&"base"));
        assert!(model_names.contains(&"small"));
        assert!(model_names.contains(&"medium"));
    }

    #[test]
    fn test_list_models_with_quantization() {
        let models_f16 = list_models_with_quantization(&ModelQuantization::F16);
        let models_q8 = list_models_with_quantization(&ModelQuantization::Q8_0);

        // Both should have same number of models
        assert_eq!(models_f16.len(), models_q8.len());

        // F16 models should not be marked as quantized
        for model in &models_f16 {
            assert!(!model.is_quantized);
        }

        // Q8_0 models should be marked as quantized
        for model in &models_q8 {
            assert!(model.is_quantized);
        }
    }

    #[test]
    fn test_list_all_model_variants() {
        let variants = list_all_model_variants();

        // Should have all sizes x all quantizations
        // 5 sizes x 3 quantizations = 15 variants
        assert!(variants.len() >= 15);

        // Verify we have different quantizations
        let quants: std::collections::HashSet<&str> =
            variants.iter().map(|m| m.quantization.as_str()).collect();
        assert!(quants.contains("f16"));
        assert!(quants.contains("q8_0"));
        assert!(quants.contains("q5_1"));
    }

    #[test]
    fn test_list_downloaded_models() {
        // Should return only downloaded models (likely empty in test env)
        let downloaded = list_downloaded_models();

        // All returned models should have downloaded = true
        for model in downloaded {
            assert!(model.downloaded);
        }
    }

    // =========================================================================
    // Model Size Tests
    // =========================================================================

    #[test]
    fn test_model_info_size_bytes_for_available_models() {
        let models = list_models();

        for model in models {
            // All models should have a positive size
            assert!(model.size_bytes > 0, "Model {} has zero size", model.model);
        }
    }

    #[test]
    fn test_model_size_increases_with_model_size() {
        let models = list_models();

        // Find tiny and medium models
        let tiny = models.iter().find(|m| m.model == "tiny");
        let medium = models.iter().find(|m| m.model == "medium");

        if let (Some(tiny), Some(medium)) = (tiny, medium) {
            assert!(medium.size_bytes > tiny.size_bytes);
        }
    }

    // =========================================================================
    // Delete Model Tests
    // =========================================================================

    #[test]
    fn test_delete_nonexistent_model() {
        // Deleting a non-existent model should not error
        let result = delete_model(&WhisperModel::Large);
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_model_with_quantization() {
        // Should not error for non-existent model
        let result = delete_model_with_quantization(&WhisperModel::Small, &ModelQuantization::Q5_1);
        assert!(result.is_ok());
    }

    // =========================================================================
    // Recommend Model Tests
    // =========================================================================

    #[test]
    fn test_recommend_model_returns_valid_model() {
        let recommended = recommend_model();

        // Should return one of the valid models
        let valid_models = WhisperModel::all();
        assert!(valid_models.contains(&recommended));
    }

    #[test]
    fn test_recommend_model_consistency() {
        // Multiple calls should return same result (system RAM doesn't change)
        let rec1 = recommend_model();
        let rec2 = recommend_model();
        assert_eq!(format!("{:?}", rec1), format!("{:?}", rec2));
    }

    // =========================================================================
    // System Memory Tests
    // =========================================================================

    #[test]
    fn test_get_system_memory() {
        let ram = get_system_memory_gb();

        // Should return Some with a reasonable value
        if let Some(gb) = ram {
            assert!(gb > 0);
            assert!(gb < 1024); // Less than 1TB
        }
    }

    // =========================================================================
    // Edge Cases Tests
    // =========================================================================

    #[test]
    fn test_model_paths_are_unique() {
        let variants = list_all_model_variants();

        let paths: Vec<_> = variants.iter().map(|m| m.path.clone()).collect();
        let unique_paths: std::collections::HashSet<_> = paths.iter().collect();

        // All paths should be unique
        assert_eq!(paths.len(), unique_paths.len());
    }

    #[test]
    fn test_quantized_models_smaller_than_f16() {
        let f16_models = list_models_with_quantization(&ModelQuantization::F16);
        let q8_models = list_models_with_quantization(&ModelQuantization::Q8_0);
        let q5_models = list_models_with_quantization(&ModelQuantization::Q5_1);

        for (f16, q8, q5) in itertools_lite(&f16_models, &q8_models, &q5_models) {
            if !f16.downloaded && !q8.downloaded && !q5.downloaded {
                // For not-downloaded models, check expected sizes
                // Q8 should be smaller than F16, Q5 should be smaller than Q8
                assert!(
                    q8.size_bytes <= f16.size_bytes,
                    "Q8 should be <= F16 for {}",
                    f16.model
                );
                assert!(
                    q5.size_bytes <= q8.size_bytes,
                    "Q5 should be <= Q8 for {}",
                    f16.model
                );
            }
        }
    }
}

// Helper for test iteration
#[cfg(test)]
fn itertools_lite<'a>(
    a: &'a [ModelInfo],
    b: &'a [ModelInfo],
    c: &'a [ModelInfo],
) -> impl Iterator<Item = (&'a ModelInfo, &'a ModelInfo, &'a ModelInfo)> {
    a.iter().zip(b.iter()).zip(c.iter()).map(|((x, y), z)| (x, y, z))
}

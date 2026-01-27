//! Model Downloader
//!
//! Download Whisper models from Hugging Face with cancellation support.
//! Supports both standard (F16) and quantized (Q8_0, Q5_1) models.
//! Includes SHA256 checksum verification to ensure model integrity.

use crate::config::{ModelQuantization, WhisperModel};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Base URL for model downloads (both standard F16 and quantized models)
const MODEL_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// SHA256 checksums for Whisper models from HuggingFace (ggerganov/whisper.cpp)
/// These checksums are from the Git LFS metadata and ensure model integrity.
///
/// Note: Medium uses Q5_0 (not Q5_1), and Large uses large-v3 variant.
mod checksums {
    use crate::config::{ModelQuantization, WhisperModel};

    /// Get the expected SHA256 checksum for a model with specific quantization
    pub fn get_checksum(model: &WhisperModel, quantization: &ModelQuantization) -> Option<&'static str> {
        match (model, quantization) {
            // Tiny models
            (WhisperModel::Tiny, ModelQuantization::F16) => {
                Some("be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21")
            }
            (WhisperModel::Tiny, ModelQuantization::Q8_0) => {
                Some("c2085835d3f50733e2ff6e4b41ae8a2b8d8110461e18821b09a15c40c42d1cca")
            }
            (WhisperModel::Tiny, ModelQuantization::Q5_1) => {
                Some("818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7")
            }

            // Base models
            (WhisperModel::Base, ModelQuantization::F16) => {
                Some("60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe")
            }
            (WhisperModel::Base, ModelQuantization::Q8_0) => {
                Some("c577b9a86e7e048a0b7eada054f4dd79a56bbfa911fbdacf900ac5b567cbb7d9")
            }
            (WhisperModel::Base, ModelQuantization::Q5_1) => {
                Some("422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898")
            }

            // Small models
            (WhisperModel::Small, ModelQuantization::F16) => {
                Some("1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b")
            }
            (WhisperModel::Small, ModelQuantization::Q8_0) => {
                Some("49c8fb02b65e6049d5fa6c04f81f53b867b5ec9540406812c643f177317f779f")
            }
            (WhisperModel::Small, ModelQuantization::Q5_1) => {
                Some("ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb")
            }

            // Medium models (Note: Medium uses Q5_0 on HuggingFace, not Q5_1)
            (WhisperModel::Medium, ModelQuantization::F16) => {
                Some("6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208")
            }
            (WhisperModel::Medium, ModelQuantization::Q8_0) => {
                Some("42a1ffcbe4167d224232443396968db4d02d4e8e87e213d3ee2e03095dea6502")
            }
            // Medium Q5_1 not available on HuggingFace (only Q5_0)
            // Q5_0 checksum: 19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f
            (WhisperModel::Medium, ModelQuantization::Q5_1) => None,

            // Large models (large-v3 variant)
            (WhisperModel::Large, ModelQuantization::F16) => {
                Some("64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2")
            }
            // Large Q8_0 not available on HuggingFace
            (WhisperModel::Large, ModelQuantization::Q8_0) => None,
            // Large Q5_0 checksum: d75795ecff3f83b5faa89d1900604ad8c780abd5739fae406de19f23ecd98ad1
            (WhisperModel::Large, ModelQuantization::Q5_1) => None,
        }
    }
}

/// Download progress callback
pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send>;

/// Download progress information
#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f32,
    pub speed_bps: u64,
}

/// Download errors
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Download cancelled")]
    Cancelled,

    #[error("Insufficient disk space: need {needed} bytes, have {available} bytes")]
    InsufficientSpace { needed: u64, available: u64 },

    #[error("Checksum verification failed: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("No checksum available for model {model} with quantization {quantization}")]
    NoChecksumAvailable { model: String, quantization: String },
}

/// Cancellation token for downloads
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Global download manager for tracking active downloads
pub struct DownloadManager {
    active_downloads: Mutex<HashMap<String, CancellationToken>>,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            active_downloads: Mutex::new(HashMap::new()),
        }
    }

    /// Start tracking a download
    pub fn start_download(&self, model: &WhisperModel) -> CancellationToken {
        let token = CancellationToken::new();
        let mut downloads = self.active_downloads.lock();
        downloads.insert(model.filename().to_string(), token.clone());
        token
    }

    /// Cancel a download
    pub fn cancel_download(&self, model: &WhisperModel) -> bool {
        let downloads = self.active_downloads.lock();
        if let Some(token) = downloads.get(model.filename()) {
            token.cancel();
            tracing::info!("Download cancelled: {}", model.filename());
            true
        } else {
            false
        }
    }

    /// Remove a completed download from tracking
    pub fn complete_download(&self, model: &WhisperModel) {
        let mut downloads = self.active_downloads.lock();
        downloads.remove(model.filename());
    }

    /// Check if a download is in progress
    pub fn is_downloading(&self, model: &WhisperModel) -> bool {
        let downloads = self.active_downloads.lock();
        downloads.contains_key(model.filename())
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard that ensures download is removed from tracking when dropped
///
/// This prevents memory leaks if the download function panics or is cancelled
struct DownloadGuard<'a> {
    manager: &'a DownloadManager,
    model_filename: String,
}

impl<'a> DownloadGuard<'a> {
    fn new(manager: &'a DownloadManager, model: &WhisperModel) -> Self {
        Self {
            manager,
            model_filename: model.filename().to_string(),
        }
    }
}

impl Drop for DownloadGuard<'_> {
    fn drop(&mut self) {
        let mut downloads = self.manager.active_downloads.lock();
        downloads.remove(&self.model_filename);
        tracing::debug!("Download guard dropped for: {}", self.model_filename);
    }
}

/// Global download manager instance
static DOWNLOAD_MANAGER: once_cell::sync::Lazy<DownloadManager> =
    once_cell::sync::Lazy::new(DownloadManager::new);

/// Get the global download manager
pub fn download_manager() -> &'static DownloadManager {
    &DOWNLOAD_MANAGER
}

/// Download a Whisper model with cancellation support (F16 by default)
pub async fn download_model(
    model: &WhisperModel,
    dest_dir: PathBuf,
    progress: Option<ProgressCallback>,
) -> Result<PathBuf, DownloadError> {
    download_model_with_quantization(model, &ModelQuantization::F16, dest_dir, progress).await
}

/// Download a Whisper model with specific quantization
pub async fn download_model_with_quantization(
    model: &WhisperModel,
    quantization: &ModelQuantization,
    dest_dir: PathBuf,
    progress: Option<ProgressCallback>,
) -> Result<PathBuf, DownloadError> {
    // Get cancellation token from manager
    let cancel_token = download_manager().start_download(model);

    // Use RAII guard to ensure cleanup happens even if we panic
    let _guard = DownloadGuard::new(download_manager(), model);

    download_model_internal_with_quantization(
        model,
        quantization,
        dest_dir,
        progress,
        &cancel_token,
    )
    .await
    // Guard is dropped here, removing the download from tracking
}

/// Check available disk space at the given path
#[cfg(windows)]
fn get_available_space(path: &std::path::Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows::core::PCWSTR;

    let path_str: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

    // SAFETY: GetDiskFreeSpaceExW is safe to call because:
    // - path_str is a valid null-terminated UTF-16 string (OsStrExt::encode_wide + null)
    // - All output parameters are valid mutable references to stack-allocated u64
    // - PCWSTR wraps a valid pointer to the path string which outlives the call
    // - We check the return value and return None on failure
    // - No resources or handles require cleanup
    unsafe {
        let mut free_bytes_available: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;

        if GetDiskFreeSpaceExW(
            PCWSTR(path_str.as_ptr()),
            Some(&mut free_bytes_available),
            Some(&mut total_bytes),
            Some(&mut total_free_bytes),
        ).is_ok() {
            Some(free_bytes_available)
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn get_available_space(_path: &std::path::Path) -> Option<u64> {
    None
}

/// Calculate SHA256 checksum of a file
///
/// Reads the file in chunks to avoid loading large models entirely into memory.
async fn calculate_file_sha256(path: &std::path::Path) -> Result<String, std::io::Error> {
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB chunks

    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Verify the SHA256 checksum of a downloaded model file
///
/// Returns Ok(()) if the checksum matches or if no checksum is available for the model.
/// Returns Err with ChecksumMismatch if verification fails.
async fn verify_model_checksum(
    path: &std::path::Path,
    model: &WhisperModel,
    quantization: &ModelQuantization,
) -> Result<(), DownloadError> {
    let expected_checksum = match checksums::get_checksum(model, quantization) {
        Some(checksum) => checksum,
        None => {
            tracing::warn!(
                "No checksum available for model {:?} with quantization {:?}, skipping verification",
                model,
                quantization
            );
            return Ok(());
        }
    };

    tracing::info!("Verifying SHA256 checksum for downloaded model...");
    let actual_checksum = calculate_file_sha256(path).await?;

    if actual_checksum != expected_checksum {
        tracing::error!(
            "Checksum mismatch! Expected: {}, Got: {}",
            expected_checksum,
            actual_checksum
        );
        return Err(DownloadError::ChecksumMismatch {
            expected: expected_checksum.to_string(),
            actual: actual_checksum,
        });
    }

    tracing::info!("Checksum verification passed: {}", actual_checksum);
    Ok(())
}

/// Internal download implementation with quantization support
async fn download_model_internal_with_quantization(
    model: &WhisperModel,
    quantization: &ModelQuantization,
    dest_dir: PathBuf,
    progress: Option<ProgressCallback>,
    cancel_token: &CancellationToken,
) -> Result<PathBuf, DownloadError> {
    // Ensure directory exists
    tokio::fs::create_dir_all(&dest_dir).await?;

    // Check disk space before downloading
    let model_size = model.size_bytes_with_quantization(quantization);
    if let Some(available) = get_available_space(&dest_dir) {
        // Need extra 10% buffer for safety
        let needed = (model_size as f64 * 1.1) as u64;
        if available < needed {
            return Err(DownloadError::InsufficientSpace {
                needed,
                available,
            });
        }
        tracing::info!("Disk space check passed: {} bytes available, {} bytes needed", available, needed);
    }

    let filename = model.filename_with_quantization(quantization);
    let url = format!("{}/{}", MODEL_BASE_URL, filename);
    let dest_path = dest_dir.join(&filename);

    tracing::info!("Downloading model from: {}", url);

    // Check for cancellation before starting
    if cancel_token.is_cancelled() {
        return Err(DownloadError::Cancelled);
    }

    // Create HTTP client
    let client = reqwest::Client::new();

    // Start download
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| DownloadError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(DownloadError::Network(format!(
            "HTTP {}: {}",
            response.status(),
            response.status().canonical_reason().unwrap_or("Unknown")
        )));
    }

    let total_bytes = response.content_length().unwrap_or(model.size_bytes());

    // Create temp file
    let temp_path = dest_path.with_extension("tmp");
    let mut file = tokio::fs::File::create(&temp_path).await?;

    // Download with progress tracking
    let mut downloaded_bytes: u64 = 0;
    let start_time = std::time::Instant::now();
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        // Check for cancellation
        if cancel_token.is_cancelled() {
            // Clean up temp file
            drop(file);
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(DownloadError::Cancelled);
        }

        let chunk = chunk_result.map_err(|e| DownloadError::Network(e.to_string()))?;

        file.write_all(&chunk).await?;
        downloaded_bytes += chunk.len() as u64;

        // Report progress
        if let Some(ref callback) = progress {
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed_bps = if elapsed > 0.0 {
                (downloaded_bytes as f64 / elapsed) as u64
            } else {
                0
            };

            callback(DownloadProgress {
                downloaded_bytes,
                total_bytes,
                percentage: (downloaded_bytes as f32 / total_bytes as f32) * 100.0,
                speed_bps,
            });
        }
    }

    // Final cancellation check before completing
    if cancel_token.is_cancelled() {
        drop(file);
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(DownloadError::Cancelled);
    }

    // Flush and close file
    file.flush().await?;
    drop(file);

    // Verify checksum before finalizing
    tracing::info!("Download complete, verifying checksum...");
    match verify_model_checksum(&temp_path, model, quantization).await {
        Ok(()) => {
            // Checksum verified, rename temp file to final name
            tokio::fs::rename(&temp_path, &dest_path).await?;
            tracing::info!("Model downloaded and verified: {:?}", dest_path);
            Ok(dest_path)
        }
        Err(e) => {
            // Checksum failed, delete the corrupted file
            tracing::error!("Checksum verification failed, deleting corrupted file");
            let _ = tokio::fs::remove_file(&temp_path).await;
            Err(e)
        }
    }
}

/// Cancel an ongoing download
pub fn cancel_download(model: &WhisperModel) -> bool {
    download_manager().cancel_download(model)
}

/// Check if a model is being downloaded
pub fn is_downloading(model: &WhisperModel) -> bool {
    download_manager().is_downloading(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_url() {
        let url = format!("{}/{}", MODEL_BASE_URL, WhisperModel::Tiny.filename());
        assert!(url.contains("ggml-tiny.bin"));
    }

    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_download_manager() {
        let manager = DownloadManager::new();
        let model = WhisperModel::Tiny;

        assert!(!manager.is_downloading(&model));

        let _token = manager.start_download(&model);
        assert!(manager.is_downloading(&model));

        manager.cancel_download(&model);
        manager.complete_download(&model);
        assert!(!manager.is_downloading(&model));
    }

    #[test]
    fn test_checksums_available_for_f16_models() {
        // All F16 models should have checksums
        assert!(checksums::get_checksum(&WhisperModel::Tiny, &ModelQuantization::F16).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Base, &ModelQuantization::F16).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Small, &ModelQuantization::F16).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Medium, &ModelQuantization::F16).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Large, &ModelQuantization::F16).is_some());
    }

    #[test]
    fn test_checksums_available_for_quantized_models() {
        // Tiny, Base, Small should have Q8_0 and Q5_1 checksums
        assert!(checksums::get_checksum(&WhisperModel::Tiny, &ModelQuantization::Q8_0).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Tiny, &ModelQuantization::Q5_1).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Base, &ModelQuantization::Q8_0).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Base, &ModelQuantization::Q5_1).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Small, &ModelQuantization::Q8_0).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Small, &ModelQuantization::Q5_1).is_some());

        // Medium has Q8_0 but not Q5_1 (only Q5_0 on HuggingFace)
        assert!(checksums::get_checksum(&WhisperModel::Medium, &ModelQuantization::Q8_0).is_some());
        assert!(checksums::get_checksum(&WhisperModel::Medium, &ModelQuantization::Q5_1).is_none());

        // Large has no quantized versions on HuggingFace
        assert!(checksums::get_checksum(&WhisperModel::Large, &ModelQuantization::Q8_0).is_none());
        assert!(checksums::get_checksum(&WhisperModel::Large, &ModelQuantization::Q5_1).is_none());
    }

    #[test]
    fn test_checksum_format() {
        // All checksums should be 64 hex characters (SHA256)
        let checksum = checksums::get_checksum(&WhisperModel::Tiny, &ModelQuantization::F16).unwrap();
        assert_eq!(checksum.len(), 64);
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_calculate_file_sha256() {
        use std::io::Write;

        // Create a temp file with known content
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.bin");

        {
            let mut file = std::fs::File::create(&file_path).unwrap();
            file.write_all(b"Hello, World!").unwrap();
        }

        // SHA256 of "Hello, World!" is known
        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        let actual = calculate_file_sha256(&file_path).await.unwrap();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_verify_model_checksum_no_checksum_available() {
        use std::io::Write;

        // Create a temp file
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_model.bin");

        {
            let mut file = std::fs::File::create(&file_path).unwrap();
            file.write_all(b"dummy content").unwrap();
        }

        // Medium Q5_1 has no checksum, so verification should pass (skip)
        let result = verify_model_checksum(
            &file_path,
            &WhisperModel::Medium,
            &ModelQuantization::Q5_1,
        ).await;

        assert!(result.is_ok());
    }
}

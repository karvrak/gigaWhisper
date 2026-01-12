//! Model Downloader
//!
//! Download Whisper models from Hugging Face with cancellation support.

use crate::config::WhisperModel;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

/// Base URL for model downloads
const MODEL_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

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

/// Global download manager instance
static DOWNLOAD_MANAGER: once_cell::sync::Lazy<DownloadManager> =
    once_cell::sync::Lazy::new(DownloadManager::new);

/// Get the global download manager
pub fn download_manager() -> &'static DownloadManager {
    &DOWNLOAD_MANAGER
}

/// Download a Whisper model with cancellation support
pub async fn download_model(
    model: &WhisperModel,
    dest_dir: PathBuf,
    progress: Option<ProgressCallback>,
) -> Result<PathBuf, DownloadError> {
    // Get cancellation token from manager
    let cancel_token = download_manager().start_download(model);

    let result = download_model_internal(model, dest_dir, progress, &cancel_token).await;

    // Remove from active downloads
    download_manager().complete_download(model);

    result
}

/// Internal download implementation with cancellation
async fn download_model_internal(
    model: &WhisperModel,
    dest_dir: PathBuf,
    progress: Option<ProgressCallback>,
    cancel_token: &CancellationToken,
) -> Result<PathBuf, DownloadError> {
    // Ensure directory exists
    tokio::fs::create_dir_all(&dest_dir).await?;

    let filename = model.filename();
    let url = format!("{}/{}", MODEL_BASE_URL, filename);
    let dest_path = dest_dir.join(filename);

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

    // Rename temp file to final name
    tokio::fs::rename(&temp_path, &dest_path).await?;

    tracing::info!("Model downloaded: {:?}", dest_path);
    Ok(dest_path)
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
}

//! Auto-update module for GigaWhisper
//!
//! Checks for updates on application startup and notifies the user.
//! Supports variant-aware updates (CPU/Vulkan/CUDA).

use crate::build_info::{BUILD_VARIANT, BUILD_VARIANT_DISPLAY};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

/// GitHub repository for update endpoints
const GITHUB_REPO: &str = "karvrak/gigaWhisper";

/// Get the update endpoint URL for the current build variant
fn get_update_endpoint() -> String {
    format!(
        "https://github.com/{}/releases/latest/download/latest-{}.json",
        GITHUB_REPO, BUILD_VARIANT
    )
}

/// Check for updates and emit an event if one is available
pub async fn check_for_updates<R: Runtime>(app: AppHandle<R>) {
    tracing::info!(
        "Checking for updates (variant: {})...",
        BUILD_VARIANT
    );
    tracing::debug!("Update endpoint: {}", get_update_endpoint());

    let endpoint = match get_update_endpoint().parse() {
        Ok(url) => url,
        Err(e) => {
            tracing::error!("Invalid update endpoint URL: {}", e);
            return;
        }
    };

    let updater = match app
        .updater_builder()
        .endpoints(vec![endpoint])
    {
        Ok(builder) => match builder.build() {
            Ok(updater) => updater,
            Err(e) => {
                tracing::warn!("Failed to build updater: {}", e);
                return;
            }
        },
        Err(e) => {
            tracing::warn!("Failed to set updater endpoints: {}", e);
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            tracing::info!(
                "Update available: {} -> {}",
                update.current_version,
                update.version
            );

            // Emit event to frontend with update info
            if let Err(e) = app.emit(
                "update-available",
                UpdateInfo {
                    current_version: update.current_version.to_string(),
                    new_version: update.version.clone(),
                    body: update.body.clone(),
                    variant: BUILD_VARIANT.to_string(),
                },
            ) {
                tracing::error!("Failed to emit update-available event: {}", e);
            }
        }
        Ok(None) => {
            tracing::info!("Application is up to date");
        }
        Err(e) => {
            tracing::warn!("Failed to check for updates: {}", e);
        }
    }
}

/// Update information sent to the frontend
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub new_version: String,
    pub body: Option<String>,
    pub variant: String,
}

/// Download and install the update
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    tracing::info!("Installing update for variant: {}", BUILD_VARIANT);

    let endpoint: tauri::Url = get_update_endpoint()
        .parse()
        .map_err(|e| format!("Invalid URL: {}", e))?;

    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;

    let update: Update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    // Emit download progress events
    let app_clone = app.clone();
    update
        .download_and_install(
            move |chunk_length, content_length: Option<u64>| {
                let progress = content_length
                    .map(|total| (chunk_length as f64 / total as f64 * 100.0) as u32);
                let _ = app_clone.emit(
                    "update-download-progress",
                    DownloadProgress {
                        downloaded: chunk_length,
                        total: content_length,
                        percent: progress,
                    },
                );
            },
            || {
                tracing::info!("Download complete, preparing to install...");
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!("Update installed, restart required");

    // Emit event to notify frontend that restart is needed
    let _ = app.emit("update-installed", ());

    Ok(())
}

/// Download progress information
#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub downloaded: usize,
    pub total: Option<u64>,
    pub percent: Option<u32>,
}

/// Restart the application to apply the update
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    tracing::info!("Restarting application to apply update...");
    app.restart();
}

/// Get the current build variant (cpu, vulkan, cuda)
#[tauri::command]
pub fn get_build_variant() -> BuildVariantInfo {
    BuildVariantInfo {
        variant: BUILD_VARIANT.to_string(),
        display_name: BUILD_VARIANT_DISPLAY.to_string(),
    }
}

/// Build variant information
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildVariantInfo {
    pub variant: String,
    pub display_name: String,
}

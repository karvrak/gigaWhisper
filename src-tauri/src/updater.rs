//! Auto-update module for GigaWhisper
//!
//! Checks for updates on application startup and notifies the user.

use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// Check for updates and emit an event if one is available
pub async fn check_for_updates(app: AppHandle) {
    tracing::info!("Checking for updates...");

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(e) => {
            tracing::warn!("Failed to initialize updater: {}", e);
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
            if let Err(e) = app.emit("update-available", UpdateInfo {
                current_version: update.current_version.to_string(),
                new_version: update.version.clone(),
                body: update.body.clone(),
            }) {
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
}

/// Download and install the update
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    tracing::info!("Installing update...");

    let updater = app.updater().map_err(|e| e.to_string())?;

    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    // Emit download progress events
    let app_clone = app.clone();
    update
        .download_and_install(
            move |chunk_length, content_length| {
                let progress = content_length.map(|total| {
                    (chunk_length as f64 / total as f64 * 100.0) as u32
                });
                let _ = app_clone.emit("update-download-progress", DownloadProgress {
                    downloaded: chunk_length,
                    total: content_length,
                    percent: progress,
                });
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

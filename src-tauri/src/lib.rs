//! GigaWhisper Library
//!
//! Core functionality for voice transcription.

pub mod audio;
pub mod build_info;
pub mod commands;
pub mod config;
pub mod history;
pub mod licensing;
pub mod llm;
pub mod models;
pub mod output;
pub mod shortcuts;
pub mod transcription;
pub mod tray;
pub mod updater;
pub mod utils;

use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Recreate the main window using the same attributes as declared in `tauri.conf.json`.
/// Used when the webview was destroyed on minimize-to-tray to release GPU resources.
pub fn recreate_main_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("GigaWhisper")
        .inner_size(400.0, 600.0)
        .min_inner_size(350.0, 500.0)
        .resizable(true)
        .decorations(true)
        .transparent(false)
        .center()
        .visible(false)
        .build()
}

/// Show the main window, recreating it if it has been destroyed.
pub fn show_or_recreate_main_window(app: &AppHandle) -> tauri::Result<()> {
    let window = match app.get_webview_window("main") {
        Some(w) => w,
        None => recreate_main_window(app)?,
    };
    window.show()?;
    window.set_focus()?;
    Ok(())
}

/// Application state shared across all components
pub struct AppState {
    pub config: parking_lot::RwLock<config::Settings>,
    pub recording_state: parking_lot::RwLock<RecordingState>,
    pub audio_capture: Mutex<Option<audio::AudioCapture>>,
    pub transcription_service: Arc<transcription::TranscriptionService>,
}

/// Current recording state
#[derive(Debug, Clone, Default)]
pub enum RecordingState {
    #[default]
    Idle,
    Recording {
        started_at: std::time::Instant,
        /// Temporary context override for this recording only (from shortcut or auto-switch).
        /// None means use the user's default `active_context` from config.
        context_override: Option<String>,
    },
    Processing,
    Error(String),
}

/// Get the log directory path
fn log_dir() -> std::path::PathBuf {
    directories::ProjectDirs::from("com", "gigawhisper", "GigaWhisper")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("logs")
}

/// Initialize logging with console and file output
/// Returns a guard that must be kept alive for the duration of the application
fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    // Initialize logging with appropriate level based on build mode
    #[cfg(debug_assertions)]
    let default_filter = "gigawhisper=debug,tauri=info";
    #[cfg(not(debug_assertions))]
    let default_filter = "gigawhisper=info,tauri=warn";

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| default_filter.into());

    // Set up file appender with daily rotation (keeps 7 days of logs)
    let log_directory = log_dir();

    // Ensure the log directory exists
    let _ = std::fs::create_dir_all(&log_directory);

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(7)
        .filename_prefix("gigawhisper")
        .filename_suffix("log")
        .build(&log_directory)
        .expect("Failed to create log file appender");

    // Create a non-blocking writer for the file appender
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Build the subscriber with both console and file output
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_ansi(true)
                .with_target(true)
                .with_thread_ids(false),
        )
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(non_blocking),
        )
        .init();

    tracing::info!("Starting GigaWhisper");
    tracing::info!("Log files stored in: {:?}", log_directory);

    guard
}

/// Initialize and run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Disable WebView2 GPU compositing to eliminate idle GPU load while minimized.
    // Must be set before the WebView2 process starts (before tauri::Builder::default()).
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--disable-gpu-compositing --disable-features=UseSkiaRenderer",
    );

    // Initialize logging - keep guard alive for the duration of the application
    let _log_guard = init_logging();

    // Check if this is the first launch (no settings file yet)
    let is_first_launch = !config::config_file().exists();
    if is_first_launch {
        tracing::info!("First launch detected - will show onboarding");
    }

    // Load configuration and sync API key flags with Credential Manager.
    // This ensures keys that survive a reinstall (stored in Windows Credential
    // Manager) are detected even if the config JSON was reset.
    let mut config = config::Settings::load().unwrap_or_default();
    config.sync_api_key_flags();
    if let Err(e) = config.save() {
        tracing::warn!("Failed to persist synced API key flags: {}", e);
    }

    // Create transcription service
    let transcription_service = Arc::new(transcription::TranscriptionService::new());

    // Create app state
    let app_state = AppState {
        config: parking_lot::RwLock::new(config.clone()),
        recording_state: parking_lot::RwLock::new(RecordingState::default()),
        audio_capture: Mutex::new(None),
        transcription_service: transcription_service.clone(),
    };

    // Update transcription service with config
    transcription_service.update_status_from_config(&config);

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .manage(app_state)
        .setup(move |app| {
            // Setup system tray
            tray::setup_tray(app)?;

            // Register global shortcuts
            shortcuts::register_shortcuts(app)?;

            // Show or hide main window on startup based on configuration
            // On first launch, always show the window for onboarding
            let state = app.state::<AppState>();
            let should_start_minimized = !is_first_launch && state.config.read().ui.start_minimized;

            if let Some(window) = app.get_webview_window("main") {
                if should_start_minimized {
                    // Destroy the pre-created (hidden) webview so it doesn't consume
                    // GPU/CPU while the app lives in the tray. It will be recreated
                    // on demand via `recreate_main_window`.
                    let keep_alive = state.config.read().ui.keep_webview_alive;
                    if keep_alive {
                        window.hide()?;
                    } else {
                        window.destroy()?;
                    }
                } else {
                    window.show()?;
                    window.set_focus()?;
                }
            }

            // Sync autostart state with settings
            {
                use tauri_plugin_autostart::ManagerExt;
                let autostart_manager = app.autolaunch();
                let auto_start_enabled = state.config.read().ui.auto_start;
                let is_enabled = autostart_manager.is_enabled().unwrap_or(false);
                if auto_start_enabled && !is_enabled {
                    if let Err(e) = autostart_manager.enable() {
                        tracing::error!("Failed to enable autostart: {}", e);
                    }
                } else if !auto_start_enabled && is_enabled {
                    if let Err(e) = autostart_manager.disable() {
                        tracing::error!("Failed to disable autostart: {}", e);
                    }
                }
                tracing::info!(
                    "Autostart state: setting={}, system={}",
                    auto_start_enabled,
                    autostart_manager.is_enabled().unwrap_or(false)
                );
            }

            // Preload Whisper model at startup only when the user has disabled idle
            // unloading (`idle_unload_after_seconds == 0`). Otherwise the model is
            // loaded lazily on shortcut press and unloaded after idle timeout.
            {
                let preload_config = state.config.read().clone();
                if preload_config.transcription.local.idle_unload_after_seconds == 0 {
                    let service = state.transcription_service.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        if let Err(e) = service.preload_model(&preload_config) {
                            tracing::warn!("Failed to preload transcription model: {}", e);
                        }
                    });
                }
            }

            // Check for updates in the background
            let app_handle = app.handle().clone();
            let auto_update = config.ui.auto_update;
            tauri::async_runtime::spawn(async move {
                // Small delay to let the app fully initialize
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                updater::check_for_updates(app_handle, auto_update).await;
            });

            tracing::info!("GigaWhisper setup complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            // Keep app alive in tray when user closes the main window.
            // Destroying the webview (vs hiding) releases the WebView2 GPU/CPU
            // resources; it is recreated on demand when the user clicks the tray.
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let keep_alive = window
                        .app_handle()
                        .state::<AppState>()
                        .config
                        .read()
                        .ui
                        .keep_webview_alive;
                    if keep_alive {
                        let _ = window.hide();
                        tracing::debug!("Main window hidden (keep_webview_alive=true)");
                    } else {
                        let _ = window.destroy();
                        tracing::debug!("Main window destroyed (will recreate on demand)");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::cancel_recording,
            commands::recording::get_recording_state,
            commands::transcription::get_transcription_status,
            commands::transcription::preload_model,
            commands::transcription::unload_model,
            commands::transcription::get_gpu_info,
            commands::transcription::get_cpu_info,
            commands::transcription::get_metrics_summary,
            commands::transcription::get_recent_metrics,
            commands::transcription::reset_metrics,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_audio_devices,
            commands::settings::set_groq_api_key,
            commands::settings::has_groq_api_key,
            commands::settings::clear_groq_api_key,
            commands::settings::validate_groq_api_key,
            commands::settings::set_openai_api_key,
            commands::settings::has_openai_api_key,
            commands::settings::clear_openai_api_key,
            commands::settings::set_deepgram_api_key,
            commands::settings::has_deepgram_api_key,
            commands::settings::clear_deepgram_api_key,
            commands::settings::set_anthropic_api_key,
            commands::settings::has_anthropic_api_key,
            commands::settings::clear_anthropic_api_key,
            commands::settings::validate_groq_api_key_live,
            commands::settings::validate_openai_api_key_live,
            commands::settings::validate_deepgram_api_key_live,
            commands::settings::validate_anthropic_api_key_live,
            commands::clipboard::paste_text,
            commands::clipboard::get_history,
            commands::models::list_models,
            commands::models::is_model_downloaded,
            commands::models::is_model_downloading,
            commands::models::download_model,
            commands::models::cancel_model_download,
            commands::models::delete_model,
            commands::models::get_recommended_model,
            commands::history::get_transcription_history,
            commands::history::get_history_entry,
            commands::history::delete_history_entry,
            commands::history::clear_history,
            commands::history::get_history_count,
            commands::history::get_audio_data,
            commands::premium::activate_license,
            commands::premium::deactivate_license,
            commands::premium::get_premium_status,
            commands::premium::check_feature,
            commands::premium::get_credits_balance,
            commands::contexts::get_contexts,
            commands::contexts::save_context,
            commands::contexts::delete_context,
            commands::contexts::set_active_context,
            commands::apps::get_running_apps,
            updater::install_update,
            updater::restart_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

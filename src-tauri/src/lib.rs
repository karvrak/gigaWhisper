//! GigaWhisper Library
//!
//! Core functionality for voice transcription.

pub mod audio;
pub mod commands;
pub mod config;
pub mod history;
pub mod models;
pub mod output;
pub mod shortcuts;
pub mod transcription;
pub mod tray;
pub mod utils;

use parking_lot::Mutex;
use std::sync::Arc;
use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    },
    Processing,
    Error(String),
}

/// Initialize and run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "gigawhisper=debug,tauri=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting GigaWhisper");

    // Check if this is the first launch (no settings file yet)
    let is_first_launch = !config::config_file().exists();
    if is_first_launch {
        tracing::info!("First launch detected - will show onboarding");
    }

    // Load configuration
    let config = config::Settings::load().unwrap_or_default();

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
                    window.hide()?;
                } else {
                    window.show()?;
                    window.set_focus()?;
                }
            }

            tracing::info!("GigaWhisper setup complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide main window instead of closing it (keep app in tray)
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // Prevent the window from being destroyed
                    api.prevent_close();
                    // Hide the window instead
                    let _ = window.hide();
                    tracing::debug!("Main window hidden (not closed)");
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

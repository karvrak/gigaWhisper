//! Tray Menu
//!
//! System tray menu construction and event handling.

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Manager,
};

/// Setup system tray
/// Uses the tray icon created from tauri.conf.json (id: "main") and adds menu + events
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Create menu items
    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    // Build menu
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    // Get existing tray icon created from tauri.conf.json (id: "main")
    let tray = app.tray_by_id("main").ok_or("Tray icon 'main' not found")?;

    // Set menu on existing tray
    tray.set_menu(Some(menu))?;
    tray.set_tooltip(Some("GigaWhisper - Voice Transcription"))?;

    // Set up menu event handler
    tray.on_menu_event(|app, event| {
        handle_menu_event(app, &event.id.0);
    });

    // Set up tray icon click event handler
    tray.on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            // Show main window on left click
            if let Some(window) = tray.app_handle().get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    });

    tracing::info!("System tray setup complete");
    Ok(())
}

/// Handle tray menu events
fn handle_menu_event(app: &tauri::AppHandle, item_id: &str) {
    match item_id {
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "quit" => {
            tracing::info!("Quit requested from tray");
            app.exit(0);
        }
        _ => {}
    }
}

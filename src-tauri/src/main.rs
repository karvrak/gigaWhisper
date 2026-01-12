//! GigaWhisper - Voice transcription application
//!
//! Entry point for the Tauri application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gigawhisper_lib::run()
}

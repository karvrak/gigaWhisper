//! Models Module
//!
//! Whisper model management and download.

mod downloader;
mod manager;

pub use downloader::*;
pub use manager::*;

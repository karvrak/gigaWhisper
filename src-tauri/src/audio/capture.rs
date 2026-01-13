//! Audio Capture
//!
//! Microphone input using cpal with thread-safe architecture.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

use super::buffer::RingBuffer;

/// Audio capture configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Target sample rate (16000 for Whisper)
    pub sample_rate: u32,
    /// Number of channels (1 for mono)
    pub channels: u16,
    /// Buffer duration in milliseconds
    pub buffer_duration_ms: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            buffer_duration_ms: 100,
        }
    }
}

/// Commands sent to the audio worker thread
enum AudioCommand {
    Start,
    Stop,
    Shutdown,
}

/// Stream error that occurred during recording
#[derive(Debug, Clone)]
pub struct StreamError {
    pub message: String,
    pub is_disconnection: bool,
}

/// Audio capture handle (Send + Sync safe)
///
/// All fields are wrapped in thread-safe containers to ensure proper
/// Send + Sync implementation without requiring unsafe code.
pub struct AudioCapture {
    /// Command sender wrapped in Mutex for Sync safety
    command_tx: Mutex<mpsc::Sender<AudioCommand>>,
    buffer: Arc<Mutex<RingBuffer>>,
    /// Worker handle wrapped in Mutex for Sync safety
    worker_handle: Mutex<Option<JoinHandle<()>>>,
    is_recording: Arc<Mutex<bool>>,
    config: AudioConfig,
    /// Actual sample rate of the device (may differ from config)
    device_sample_rate: u32,
    /// Last stream error (if any)
    last_error: Arc<Mutex<Option<StreamError>>>,
}

// AudioCapture is now automatically Send + Sync because:
// - Mutex<mpsc::Sender<T>> is Send + Sync when T: Send
// - Arc<Mutex<T>> is Send + Sync when T: Send
// - Mutex<Option<JoinHandle<()>>> is Send + Sync
// - AudioConfig is Send + Sync (contains only primitive types)
// - u32 is Send + Sync

/// Audio device information
#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Audio capture errors
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("No audio host available")]
    NoHost,

    #[error("No default input device")]
    NoDefaultDevice,

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Failed to get device config: {0}")]
    ConfigError(String),

    #[error("Failed to build stream: {0}")]
    StreamError(String),

    #[error("Stream error: {0}")]
    PlayError(String),

    #[error("Worker thread error")]
    WorkerError,
}

impl AudioCapture {
    /// Create a new audio capture with default device
    pub fn new(config: AudioConfig) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(AudioError::NoDefaultDevice)?;

        Self::with_device(device, config)
    }

    /// Create audio capture with specific device
    pub fn with_device(device: cpal::Device, config: AudioConfig) -> Result<Self, AudioError> {
        let supported_config = device
            .default_input_config()
            .map_err(|e| AudioError::ConfigError(e.to_string()))?;

        let device_sample_rate = supported_config.sample_rate().0;
        let device_config = cpal::StreamConfig {
            channels: supported_config.channels(),
            sample_rate: supported_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        tracing::info!(
            "Audio device: {}Hz, {} channels",
            device_sample_rate,
            supported_config.channels()
        );

        // Calculate buffer size based on config duration
        // buffer_duration_ms is in milliseconds, convert to seconds
        let buffer_seconds = (config.buffer_duration_ms as f64 / 1000.0).max(60.0); // Minimum 60s
        let buffer_samples = (device_sample_rate as f64 * buffer_seconds) as usize;
        tracing::info!("Audio buffer size: {}s ({} samples)", buffer_seconds, buffer_samples);
        let buffer = Arc::new(Mutex::new(RingBuffer::new(buffer_samples)));
        let is_recording = Arc::new(Mutex::new(false));
        let last_error: Arc<Mutex<Option<StreamError>>> = Arc::new(Mutex::new(None));

        // Create channel for commands
        let (command_tx, command_rx) = mpsc::channel::<AudioCommand>();

        // Clone references for the worker thread
        let buffer_clone = buffer.clone();
        let is_recording_clone = is_recording.clone();
        let last_error_clone = last_error.clone();
        let channels = device_config.channels as usize;

        // Spawn worker thread that owns the device and stream
        let worker_handle = std::thread::spawn(move || {
            let mut stream: Option<cpal::Stream> = None;

            loop {
                match command_rx.recv() {
                    Ok(AudioCommand::Start) => {
                        if stream.is_some() {
                            continue; // Already recording
                        }

                        // Clear any previous error
                        *last_error_clone.lock() = None;

                        let buffer_for_callback = buffer_clone.clone();
                        let channels_for_callback = channels;
                        let error_for_callback = last_error_clone.clone();
                        let is_recording_for_error = is_recording_clone.clone();

                        match device.build_input_stream(
                            &device_config,
                            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                // Convert to mono if stereo
                                let mono: Vec<f32> = if channels_for_callback > 1 {
                                    data.chunks(channels_for_callback)
                                        .map(|chunk| {
                                            chunk.iter().sum::<f32>() / channels_for_callback as f32
                                        })
                                        .collect()
                                } else {
                                    data.to_vec()
                                };

                                let mut buf = buffer_for_callback.lock();
                                buf.write(&mono);
                            },
                            move |err| {
                                let error_msg = err.to_string();
                                tracing::error!("Audio stream error: {}", error_msg);

                                // Detect disconnection errors
                                let is_disconnection = error_msg.contains("disconnected")
                                    || error_msg.contains("device")
                                    || error_msg.contains("DeviceNotAvailable")
                                    || error_msg.contains("lost")
                                    || error_msg.contains("InvalidDevice");

                                // Store the error
                                *error_for_callback.lock() = Some(StreamError {
                                    message: error_msg,
                                    is_disconnection,
                                });

                                // Mark as no longer recording on critical errors
                                if is_disconnection {
                                    *is_recording_for_error.lock() = false;
                                }
                            },
                            None,
                        ) {
                            Ok(s) => {
                                if s.play().is_ok() {
                                    *is_recording_clone.lock() = true;
                                    stream = Some(s);
                                    tracing::info!("Audio capture started");
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to build audio stream: {}", e);
                                *last_error_clone.lock() = Some(StreamError {
                                    message: e.to_string(),
                                    is_disconnection: true,
                                });
                            }
                        }
                    }
                    Ok(AudioCommand::Stop) => {
                        stream = None; // Drop the stream to stop capture
                        *is_recording_clone.lock() = false;
                        tracing::info!("Audio capture stopped");
                    }
                    Ok(AudioCommand::Shutdown) | Err(_) => {
                        drop(stream.take()); // Explicitly drop stream to stop capture
                        *is_recording_clone.lock() = false;
                        break;
                    }
                }
            }
        });

        Ok(Self {
            command_tx: Mutex::new(command_tx),
            buffer,
            worker_handle: Mutex::new(Some(worker_handle)),
            is_recording,
            config,
            last_error,
            device_sample_rate,
        })
    }

    /// List available input devices
    pub fn list_devices() -> Result<Vec<AudioDevice>, AudioError> {
        let host = cpal::default_host();
        let default_device = host.default_input_device();
        let default_name = default_device.as_ref().and_then(|d| d.name().ok());

        let devices: Vec<AudioDevice> = host
            .input_devices()
            .map_err(|_| AudioError::NoHost)?
            .filter_map(|device| {
                let name = device.name().ok()?;
                Some(AudioDevice {
                    id: name.clone(),
                    name: name.clone(),
                    is_default: Some(&name) == default_name.as_ref(),
                })
            })
            .collect();

        Ok(devices)
    }

    /// Start capturing audio
    pub fn start(&self) -> Result<(), AudioError> {
        self.command_tx
            .lock()
            .send(AudioCommand::Start)
            .map_err(|_| AudioError::WorkerError)?;

        // Give the worker thread a moment to start
        std::thread::sleep(std::time::Duration::from_millis(50));

        Ok(())
    }

    /// Stop capturing and return captured samples with the device sample rate
    pub fn stop(&self) -> Result<(Vec<f32>, u32), AudioError> {
        self.command_tx
            .lock()
            .send(AudioCommand::Stop)
            .map_err(|_| AudioError::WorkerError)?;

        // Give the worker thread a moment to stop
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Get all samples from buffer
        let mut buffer = self.buffer.lock();
        let samples = buffer.drain();

        tracing::info!(
            "Audio capture stopped, {} samples collected at {}Hz",
            samples.len(),
            self.device_sample_rate
        );

        Ok((samples, self.device_sample_rate))
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock()
    }

    /// Clear the buffer without stopping
    pub fn clear(&self) {
        let mut buffer = self.buffer.lock();
        buffer.clear();
    }

    /// Get the audio config
    pub fn config(&self) -> &AudioConfig {
        &self.config
    }

    /// Get the actual device sample rate
    pub fn device_sample_rate(&self) -> u32 {
        self.device_sample_rate
    }

    /// Check if there was a stream error (e.g., microphone disconnected)
    pub fn has_error(&self) -> bool {
        self.last_error.lock().is_some()
    }

    /// Get the last stream error, if any
    pub fn get_error(&self) -> Option<StreamError> {
        self.last_error.lock().clone()
    }

    /// Clear the last error
    pub fn clear_error(&self) {
        *self.last_error.lock() = None;
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        // Send shutdown command to worker thread
        let _ = self.command_tx.lock().send(AudioCommand::Shutdown);

        // Wait for worker thread to finish
        if let Some(handle) = self.worker_handle.lock().take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        // This test may fail on CI without audio devices
        let result = AudioCapture::list_devices();
        // Just check it doesn't panic
        let _ = result;
    }
}

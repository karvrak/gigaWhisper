# ADR-003: Audio Capture with cpal

## Status
Accepted

## Context

GigaWhisper needs to capture audio from the user's microphone with:
- Minimal latency (< 50ms)
- Support for all Windows audio devices
- Whisper-compatible format (16kHz, mono, f32)
- Proper error handling (microphone unplugged, etc.)

Options evaluated:
1. **cpal** - Cross-platform audio I/O in Rust
2. **rodio** - High-level, based on cpal
3. **Windows Audio Session API (WASAPI)** direct
4. **PortAudio** bindings

## Decision

Use **cpal** (Cross-Platform Audio Library) for microphone capture.

## Implementation

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioCapture {
    stream: Option<cpal::Stream>,
    buffer: Arc<Mutex<RingBuffer<f32>>>,
    config: AudioConfig,
}

pub struct AudioConfig {
    pub sample_rate: u32,      // 16000 Hz for whisper
    pub channels: u16,         // 1 (mono)
    pub buffer_duration_ms: u32, // 100ms ring buffer
}

impl AudioCapture {
    pub fn new(device: Option<String>) -> Result<Self> {
        let host = cpal::default_host();
        let device = match device {
            Some(name) => host.input_devices()?
                .find(|d| d.name().ok() == Some(name.clone()))
                .ok_or(AudioError::DeviceNotFound)?,
            None => host.default_input_device()
                .ok_or(AudioError::NoDefaultDevice)?,
        };
        // ...
    }

    pub fn start(&mut self) -> Result<()>;
    pub fn stop(&mut self) -> Result<()>;
    pub fn get_samples(&self) -> Vec<f32>;
}
```

### Audio Pipeline

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Microphone  │───▶│    cpal      │───▶│  Resampler   │───▶│ Ring Buffer  │
│  (native)    │    │  (callback)  │    │  (if needed) │    │   (f32)      │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
                           │                                        │
                           ▼                                        ▼
                    ┌──────────────┐                        ┌──────────────┐
                    │   48kHz      │                        │   16kHz      │
                    │   Stereo     │                        │    Mono      │
                    │   f32        │                        │    f32       │
                    └──────────────┘                        └──────────────┘
```

## Consequences

### Positives
- **Pure Rust**: No external C dependencies to manage
- **Cross-platform**: Works on Windows, macOS, Linux
- **Low-level**: Full control over format and latency
- **Native WASAPI**: Uses WASAPI on Windows (optimal)
- **Async-friendly**: Non-blocking callbacks

### Negatives
- **Verbose**: More code than rodio for simple cases
- **Manual resampling**: Must implement 48kHz -> 16kHz conversion
- **Error handling**: Must manually handle device disconnection

## Target Audio Configuration

```rust
const WHISPER_SAMPLE_RATE: u32 = 16000;
const WHISPER_CHANNELS: u16 = 1;

// Intermediate format (capture)
// Most microphones are at 48kHz stereo
// We resample to 16kHz mono for whisper
```

## Resampling

Using `rubato` crate for quality resampling:

```rust
use rubato::{Resampler, SincFixedIn, SincInterpolationType};

fn create_resampler(from_rate: u32, to_rate: u32) -> SincFixedIn<f32> {
    SincFixedIn::new(
        to_rate as f64 / from_rate as f64,
        2.0,  // max relative ratio
        SincInterpolationType::Linear,
        256,  // chunk size
        1,    // channels
    ).unwrap()
}
```

## Alternatives Considered

### rodio
- **Rejected because**: Playback-oriented, less control over input
- **Advantage**: Simpler API

### WASAPI direct
- **Rejected because**: Windows-only, more complex
- **Advantage**: Potentially lower latency

### PortAudio
- **Rejected because**: Less maintained Rust bindings, C dependency
- **Advantage**: Very mature, well-documented

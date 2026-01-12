# ADR-003: Capture Audio avec cpal

## Status
Accepted

## Context

GigaWhisper doit capturer l'audio du microphone de l'utilisateur avec :
- Latence minimale (< 50ms)
- Support de tous les peripheriques audio Windows
- Format compatible whisper (16kHz, mono, f32)
- Gestion propre des erreurs (micro debranche, etc.)

Options evaluees :
1. **cpal** - Cross-platform audio I/O en Rust
2. **rodio** - Haut niveau, base sur cpal
3. **Windows Audio Session API (WASAPI)** direct
4. **PortAudio** bindings

## Decision

Utiliser **cpal** (Cross-Platform Audio Library) pour la capture microphone.

## Implementation

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioCapture {
    stream: Option<cpal::Stream>,
    buffer: Arc<Mutex<RingBuffer<f32>>>,
    config: AudioConfig,
}

pub struct AudioConfig {
    pub sample_rate: u32,      // 16000 Hz pour whisper
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

### Pipeline Audio

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
- **Pure Rust** : Pas de dependances C externes a gerer
- **Cross-platform** : Fonctionne sur Windows, macOS, Linux
- **Bas niveau** : Controle total sur le format et la latence
- **WASAPI natif** : Utilise WASAPI sur Windows (optimal)
- **Async-friendly** : Callbacks non-bloquants

### Negatives
- **Verbeux** : Plus de code que rodio pour cas simples
- **Resampling manuel** : Doit implementer conversion 48kHz -> 16kHz
- **Gestion erreurs** : Doit gerer deconnexion device manuellement

## Configuration Audio Cible

```rust
const WHISPER_SAMPLE_RATE: u32 = 16000;
const WHISPER_CHANNELS: u16 = 1;

// Format intermediaire (capture)
// La plupart des micros sont en 48kHz stereo
// On resample vers 16kHz mono pour whisper
```

## Resampling

Utilisation de `rubato` crate pour resampling de qualite :

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
- **Rejete car** : Oriente playback, moins de controle sur input
- **Avantage** : API plus simple

### WASAPI direct
- **Rejete car** : Windows-only, plus complexe
- **Avantage** : Latence potentiellement plus faible

### PortAudio
- **Rejete car** : Bindings Rust moins maintenus, dependance C
- **Avantage** : Tres mature, bien documente

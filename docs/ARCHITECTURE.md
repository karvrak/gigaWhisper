# GigaWhisper - Architecture Document

## Overview

GigaWhisper est une application desktop Windows open-source pour la transcription vocale en temps reel. Elle permet de dicter du texte qui est automatiquement insere dans n'importe quelle application.

## Architecture Principles

### 1. Performance First
- Latence cible < 500ms entre fin de parole et texte affiche
- Utilisation memoire < 200MB idle, < 500MB pendant transcription
- Demarrage rapide < 2s

### 2. Privacy by Default
- Transcription locale par defaut (whisper.cpp)
- Option cloud explicite (Groq API)
- Aucune telemetrie sans consentement

### 3. Simplicity
- Une seule action : appuyer sur le raccourci et parler
- Configuration minimale requise
- Fonctionne "out of the box"

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                                  GIGAWHISPER                                     │
│                                                                                  │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                           USER INTERFACE LAYER                              │ │
│  │                                                                             │ │
│  │   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌────────────┐  │ │
│  │   │   System     │   │  Recording   │   │   Settings   │   │   Popup    │  │ │
│  │   │    Tray      │   │  Indicator   │   │    Panel     │   │  Overlay   │  │ │
│  │   │              │   │              │   │              │   │            │  │ │
│  │   │  - Menu      │   │  - Waveform  │   │  - Hotkeys   │   │  - Text    │  │ │
│  │   │  - Status    │   │  - Timer     │   │  - Provider  │   │  - Copy    │  │ │
│  │   │  - Quick     │   │  - Cancel    │   │  - Models    │   │  - Close   │  │ │
│  │   │    actions   │   │              │   │  - Audio     │   │            │  │ │
│  │   └──────┬───────┘   └──────┬───────┘   └──────┬───────┘   └─────┬──────┘  │ │
│  │          │                  │                  │                 │         │ │
│  └──────────┼──────────────────┼──────────────────┼─────────────────┼─────────┘ │
│             │                  │                  │                 │           │
│             └──────────────────┼──────────────────┼─────────────────┘           │
│                                │                  │                             │
│                                ▼                  ▼                             │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                          APPLICATION LAYER                                  │ │
│  │                                                                             │ │
│  │   ┌─────────────────────────────────────────────────────────────────────┐  │ │
│  │   │                        Tauri Commands (IPC)                         │  │ │
│  │   │                                                                     │  │ │
│  │   │  recording::start()    transcription::get_status()                  │  │ │
│  │   │  recording::stop()     settings::get()                              │  │ │
│  │   │  recording::cancel()   settings::save()                             │  │ │
│  │   │                        clipboard::paste()                           │  │ │
│  │   └─────────────────────────────────────────────────────────────────────┘  │ │
│  │                                      │                                      │ │
│  │   ┌──────────────────┐  ┌───────────┴───────────┐  ┌──────────────────┐   │ │
│  │   │    Recording     │  │    Transcription      │  │     Output       │   │ │
│  │   │   Controller     │  │     Orchestrator      │  │    Manager       │   │ │
│  │   │                  │  │                       │  │                  │   │ │
│  │   │ - State machine  │  │ - Provider selection  │  │ - Focus detect   │   │ │
│  │   │ - Mode handling  │  │ - Audio preprocessing │  │ - Paste strategy │   │ │
│  │   │ - Timeout mgmt   │  │ - Result caching      │  │ - History save   │   │ │
│  │   └────────┬─────────┘  └───────────┬───────────┘  └────────┬─────────┘   │ │
│  │            │                        │                       │             │ │
│  └────────────┼────────────────────────┼───────────────────────┼─────────────┘ │
│               │                        │                       │               │
│               ▼                        ▼                       ▼               │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                         INFRASTRUCTURE LAYER                                │ │
│  │                                                                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │ │
│  │  │   Audio     │  │  Whisper    │  │    Groq     │  │  Keyboard   │       │ │
│  │  │  Capture    │  │   .cpp      │  │   Client    │  │  Injector   │       │ │
│  │  │   (cpal)    │  │  (FFI)      │  │  (reqwest)  │  │  (winapi)   │       │ │
│  │  │             │  │             │  │             │  │             │       │ │
│  │  │ - Device    │  │ - Model     │  │ - Auth      │  │ - SendInput │       │ │
│  │  │   enum      │  │   loading   │  │ - Retry     │  │ - Clipboard │       │ │
│  │  │ - Resample  │  │ - Inference │  │ - Timeout   │  │ - Focus     │       │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘       │ │
│  │                                                                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                        │ │
│  │  │   Global    │  │   Config    │  │   Model     │                        │ │
│  │  │  Shortcuts  │  │   Store     │  │  Manager    │                        │ │
│  │  │  (plugin)   │  │  (serde)    │  │ (download)  │                        │ │
│  │  │             │  │             │  │             │                        │ │
│  │  │ - Register  │  │ - JSON file │  │ - Progress  │                        │ │
│  │  │ - Handler   │  │ - Defaults  │  │ - Verify    │                        │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                        │ │
│  │                                                                             │ │
│  └────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Audio Capture Module

**Responsabilite** : Capturer l'audio du microphone et le convertir au format whisper.

```rust
pub struct AudioCapture {
    device: cpal::Device,
    stream: Option<cpal::Stream>,
    buffer: Arc<Mutex<RingBuffer>>,
    config: AudioConfig,
}

pub trait AudioCapturePort {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<Vec<f32>>;
    fn get_devices(&self) -> Vec<AudioDevice>;
    fn set_device(&mut self, device_id: &str) -> Result<()>;
}
```

**Format de sortie** : 16kHz, mono, f32 (normalise -1.0 a 1.0)

### 2. Transcription Module

**Responsabilite** : Convertir l'audio en texte via le provider configure.

```rust
#[async_trait]
pub trait TranscriptionProvider: Send + Sync {
    async fn transcribe(&self, audio: &[f32], config: &TranscriptionConfig) -> Result<TranscriptionResult>;
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
}

pub struct TranscriptionOrchestrator {
    primary: Box<dyn TranscriptionProvider>,
    fallback: Option<Box<dyn TranscriptionProvider>>,
}
```

### 3. Output Module

**Responsabilite** : Inserer le texte transcrit dans l'application active.

```rust
pub enum OutputStrategy {
    Clipboard,      // Ctrl+V
    TypeCharacters, // SendInput character by character
    Popup,          // Show overlay window
}

pub struct OutputManager {
    strategy: OutputStrategy,
    history: Vec<TranscriptionEntry>,
}
```

### 4. Shortcuts Module

**Responsabilite** : Gerer les raccourcis clavier globaux.

```rust
pub struct ShortcutManager {
    shortcuts: HashMap<String, ShortcutAction>,
}

pub enum ShortcutAction {
    StartRecording,
    StopRecording,
    ToggleRecording,
    CancelRecording,
    OpenSettings,
}
```

## Data Flow

### Recording Flow (Push-to-Talk Mode)

```
1. User presses Ctrl+Space (key down)
   │
   ▼
2. ShortcutManager receives event
   │
   ▼
3. RecordingController.start()
   │
   ├──▶ AudioCapture.start()
   │    └──▶ cpal stream begins
   │
   └──▶ UI shows RecordingIndicator

4. User releases Ctrl+Space (key up)
   │
   ▼
5. RecordingController.stop()
   │
   ├──▶ AudioCapture.stop() → returns Vec<f32>
   │
   └──▶ TranscriptionOrchestrator.transcribe(audio)
        │
        ├──▶ [Local] WhisperProvider.transcribe()
        │    └──▶ whisper.cpp inference
        │
        └──▶ [Cloud] GroqProvider.transcribe()
             └──▶ HTTP POST to api.groq.com

6. TranscriptionResult received
   │
   ▼
7. OutputManager.output(text)
   │
   ├──▶ FocusDetector.has_text_input()?
   │    │
   │    ├──▶ YES: KeyboardInjector.paste_via_clipboard()
   │    │
   │    └──▶ NO: PopupOverlay.show(text)
   │
   └──▶ HistoryStore.save(entry)
```

### Toggle Mode Flow

```
1. User presses Ctrl+Space (toggle on)
   │
   ▼
2. RecordingController.toggle() → state = Recording
   │
   └──▶ Same as PTT start

3. User presses Ctrl+Space again (toggle off)
   │
   ▼
4. RecordingController.toggle() → state = Processing
   │
   └──▶ Same as PTT stop
```

## State Machine

```
                    ┌─────────┐
                    │  IDLE   │◀─────────────────────────┐
                    └────┬────┘                          │
                         │                               │
            Hotkey Press │                               │
                         ▼                               │
                    ┌─────────┐                          │
              ┌────▶│RECORDING│────────┐                 │
              │     └────┬────┘        │                 │
              │          │             │                 │
    (Toggle)  │  Hotkey  │             │ Cancel          │
    Hotkey    │  Release │             │ or Error        │
              │          ▼             │                 │
              │     ┌──────────┐       │                 │
              └─────│PROCESSING│───────┤                 │
                    └────┬─────┘       │                 │
                         │             │                 │
           Transcription │             │                 │
               Complete  │             │                 │
                         ▼             ▼                 │
                    ┌─────────┐   ┌─────────┐           │
                    │OUTPUTING│   │ ERROR   │───────────┤
                    └────┬────┘   └─────────┘           │
                         │                              │
              Paste Done │                              │
                         └──────────────────────────────┘
```

## Configuration Schema

```typescript
interface GigaWhisperConfig {
  // Recording
  recording: {
    mode: 'push-to-talk' | 'toggle';
    maxDuration: number;        // seconds, 0 = unlimited
    silenceTimeout: number;     // auto-stop after silence (ms)
  };

  // Shortcuts
  shortcuts: {
    record: string;             // default: "Ctrl+Space"
    cancel: string;             // default: "Escape"
    settings: string;           // default: "Ctrl+Shift+W"
  };

  // Transcription
  transcription: {
    provider: 'local' | 'groq';
    language: string | 'auto';  // ISO 639-1 code

    // Local (whisper.cpp)
    local: {
      model: 'tiny' | 'base' | 'small' | 'medium' | 'large';
      threads: number;
      gpuEnabled: boolean;
    };

    // Cloud (Groq)
    groq: {
      apiKey: string;           // encrypted at rest
      model: string;
    };
  };

  // Audio
  audio: {
    inputDevice: string | null; // null = default
    sampleRate: number;         // always 16000 for whisper
  };

  // Output
  output: {
    autoCapitalize: boolean;
    autoPunctuation: boolean;
    pasteDelay: number;         // ms before paste
  };

  // UI
  ui: {
    showIndicator: boolean;
    indicatorPosition: 'cursor' | 'center' | 'corner';
    theme: 'system' | 'light' | 'dark';
    startMinimized: boolean;
    minimizeToTray: boolean;
  };
}
```

## Security Considerations

### API Key Storage
- Groq API key stored encrypted using Windows DPAPI
- Never logged or transmitted except to Groq API

### Permissions
- Microphone access (explicit Windows permission)
- Keyboard hooks (for global shortcuts)
- Clipboard access (for paste functionality)

### Data Privacy
- Audio never saved to disk (memory only)
- Transcription history stored locally only
- No telemetry without explicit opt-in

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Startup time | < 2s | Cold start to ready |
| Recording latency | < 50ms | Hotkey to audio capture |
| Local transcription | < 2s/10s audio | Using 'base' model |
| Cloud transcription | < 1s/30s audio | Groq API |
| Memory (idle) | < 100MB | No model loaded |
| Memory (recording) | < 200MB | With audio buffer |
| Memory (transcribing) | < 500MB | Model loaded |
| Bundle size | < 20MB | Without models |

## Technology Stack Summary

| Layer | Technology | Justification |
|-------|------------|---------------|
| Framework | Tauri v2 | Performance, bundle size, Rust ecosystem |
| Backend | Rust | Memory safety, native performance |
| Frontend | React + TypeScript | Developer productivity, ecosystem |
| Build | Vite | Fast HMR, optimized builds |
| Styling | Tailwind CSS | Rapid UI development |
| State | Zustand | Simple, performant |
| Audio | cpal | Cross-platform, low-level control |
| Transcription (local) | whisper-rs | Safe bindings to whisper.cpp |
| Transcription (cloud) | reqwest | Async HTTP client |
| Windows API | windows-rs | Official Microsoft bindings |
| Packaging | tauri-bundler | MSI/EXE installer |

## Future Considerations

### Phase 2 Features
- [ ] Voice commands ("delete that", "new line")
- [ ] Custom vocabulary/names
- [ ] Multi-language support in single session
- [ ] Audio preprocessing (noise reduction)

### Phase 3 Features
- [ ] macOS support (Tauri is cross-platform)
- [ ] Linux support
- [ ] Plugin system for custom providers
- [ ] Real-time streaming transcription

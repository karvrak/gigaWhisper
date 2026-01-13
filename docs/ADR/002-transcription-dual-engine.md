# ADR-002: Dual-Engine Architecture for Transcription

## Status
Accepted

## Context

GigaWhisper needs to transcribe voice audio to text. Two approaches exist:

1. **Local**: whisper.cpp running on the user's machine
2. **Cloud**: External API (Groq, OpenAI Whisper API, etc.)

Users have varied needs:
- Some prioritize **privacy** (all local)
- Others want **maximum quality** without a powerful GPU
- **Response time** is critical for user experience

## Decision

Implement a **dual-engine** architecture with:
1. **whisper.cpp** (local) as the default engine
2. **Groq API** (cloud) as a high-performance option

The user can choose their provider in settings. Automatic fallback is possible if the primary provider fails.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              TranscriptionOrchestrator                   │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │           trait TranscriptionProvider            │    │
│  │  + transcribe(audio: &[f32]) -> Result<String>  │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                               │
│            ┌─────────────┴─────────────┐                │
│            ▼                           ▼                │
│  ┌─────────────────┐        ┌─────────────────┐        │
│  │ WhisperProvider │        │   GroqProvider  │        │
│  │                 │        │                 │        │
│  │ - whisper.cpp   │        │ - REST API      │        │
│  │ - Local models  │        │ - API Key       │        │
│  │ - CPU/GPU       │        │ - Rate limits   │        │
│  └─────────────────┘        └─────────────────┘        │
└─────────────────────────────────────────────────────────┘
```

## Consequences

### Positives
- **Flexibility**: User chooses based on their priorities
- **Resilience**: Fallback if a provider is unavailable
- **Extensibility**: Easy to add other providers (OpenAI, local LLM)
- **Offline capable**: Local mode works without internet

### Negatives
- **Complexity**: Two implementations to maintain
- **Local models**: Initial download (75MB - 1.5GB depending on model)
- **Configuration**: More options for the user

## Provider Details

### whisper.cpp (Local)
```rust
// Exposed configuration
struct WhisperConfig {
    model: WhisperModel,      // tiny, base, small, medium, large
    language: Option<String>, // auto-detect or force
    translate: bool,          // translate to English
    threads: usize,           // CPU parallelism
    gpu: bool,                // GPU acceleration if available
}
```

**Supported models**:
| Model  | Size   | VRAM  | Quality   |
|--------|--------|-------|-----------|
| tiny   | 75 MB  | ~1GB  | Basic     |
| base   | 142 MB | ~1GB  | Fair      |
| small  | 466 MB | ~2GB  | Good      |
| medium | 1.5 GB | ~5GB  | Very good |
| large  | 2.9 GB | ~10GB | Excellent |

### Groq API (Cloud)
```rust
struct GroqConfig {
    api_key: String,
    model: String,           // whisper-large-v3
    response_format: String, // json, text, verbose_json
}
```

**Groq advantages**:
- Ultra-low latency (~0.5s for 30s audio)
- large-v3 model without local GPU
- 100 requests/day free

## Alternatives Considered

### OpenAI Whisper API alone
- **Rejected because**: Cost ($0.006/minute), no offline mode
- **Advantage**: Stable, well-documented API

### whisper.cpp alone
- **Rejected because**: Limited quality on weak CPUs, no GPU = slow
- **Advantage**: 100% offline, privacy

### Faster-whisper (Python)
- **Rejected because**: Requires Python runtime, complicates packaging
- **Advantage**: Faster than whisper.cpp on some setups

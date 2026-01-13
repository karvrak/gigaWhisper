# ADR-001: Desktop Framework Choice - Tauri v2

## Status
Accepted

## Context

GigaWhisper requires a desktop framework for Windows with the following requirements:
- Native performance (minimal latency for voice transcription)
- Easy integration with whisper.cpp (C++ library)
- Reliable global shortcuts
- Native system tray
- Keyboard injection (automatic paste)
- Lightweight bundle for open-source distribution

The options evaluated were:
1. **Tauri** (Rust + WebView)
2. **Electron** (Node.js + Chromium)
3. **Flutter Desktop** (Dart)
4. **.NET WPF/MAUI** (C#)

## Decision

We choose **Tauri v2** with Rust as the backend and React/TypeScript for the UI.

## Consequences

### Positives
- **Performance**: Rust compiles to native code, no GC, minimal overhead
- **Bundle size**: ~5-10 MB vs ~150 MB for Electron
- **Security**: Rust memory safety, no buffer overflow vulnerabilities
- **whisper.cpp integration**: Direct C/Rust FFI via `whisper-rs`
- **Mature ecosystem**: Official plugins for shortcuts, tray, clipboard
- **Open-source friendly**: MIT license, active community (80k+ stars)

### Negatives
- **Learning curve**: Rust is more complex than JavaScript/C#
- **WebView variability**: Depends on WebView2 on Windows (pre-installed on W10/W11)
- **Debugging**: Less mature tools than Electron DevTools

### Mitigated Risks
- WebView2: Pre-installed on Windows 10/11, fallback installer if absent
- Rust complexity: Use of mature crates, idiomatic patterns

## Alternatives Considered

### Electron
- **Rejected because**: Bundle too heavy (150MB+), excessive RAM consumption (100MB+ idle)
- **Advantage not retained**: More accessible JavaScript ecosystem

### Flutter Desktop
- **Rejected because**: Complex C++ integration via Dart FFI, less native Windows support
- **Advantage not retained**: Modern declarative UI

### .NET WPF
- **Rejected because**: Less suited for open-source, verbose C++ interop
- **Advantage not retained**: Perfect Windows integration

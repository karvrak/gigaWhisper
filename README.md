# GigaWhisper

Open-source voice transcription for Windows. A SuperWhisper alternative built with Tauri.

## Features

- **Global Hotkey**: Press Ctrl+Space to start dictating
- **Push-to-Talk or Toggle**: Choose your preferred recording mode
- **Local Transcription**: Run whisper.cpp on your machine (private, offline)
- **Cloud Transcription**: Use Groq API for fast, high-quality results
- **Auto-Paste**: Text is automatically pasted into the active field
- **Lightweight**: ~10MB installer, minimal resource usage

## Installation

### From Release

Download the latest installer from [Releases](https://github.com/yourusername/gigawhisper/releases).

### Build from Source

#### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Node.js](https://nodejs.org/) (18+)
- [pnpm](https://pnpm.io/) (8+)

#### Steps

```bash
# Clone the repository
git clone https://github.com/yourusername/gigawhisper.git
cd gigawhisper

# Install frontend dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

## Usage

1. **Start GigaWhisper** - It will minimize to the system tray
2. **Configure** - Click the tray icon to open settings
3. **Record** - Press `Ctrl+Space` (default) to start recording
4. **Speak** - Your voice will be transcribed
5. **Auto-paste** - Text appears in the active field

### Recording Modes

- **Push-to-Talk**: Hold the hotkey while speaking, release to transcribe
- **Toggle**: Press once to start, press again to stop and transcribe

### Transcription Providers

#### Local (whisper.cpp)

Runs entirely on your machine. Choose a model based on your hardware:

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| Tiny | 75 MB | Fastest | Basic |
| Base | 142 MB | Fast | Good |
| Small | 466 MB | Moderate | Better |
| Medium | 1.5 GB | Slow | Great |
| Large | 2.9 GB | Slowest | Best |

#### Cloud (Groq)

Fast cloud transcription using Groq's Whisper API:

1. Get an API key from [console.groq.com](https://console.groq.com)
2. Enter your key in Settings > Transcription
3. Select "Groq Cloud" as provider

## Keyboard Shortcuts

| Action | Default | Configurable |
|--------|---------|--------------|
| Record | Ctrl+Space | Yes |
| Cancel | Escape | Yes |
| Settings | Ctrl+Shift+W | Yes |

## Configuration

Settings are stored in:
- Windows: `%APPDATA%\GigaWhisper\config\settings.toml`

## Architecture

Built with:
- **[Tauri v2](https://tauri.app/)** - Rust-based desktop framework
- **[whisper-rs](https://github.com/tazz4843/whisper-rs)** - Rust bindings for whisper.cpp
- **[React](https://react.dev/)** - UI framework
- **[Tailwind CSS](https://tailwindcss.com/)** - Styling

See [Architecture Documentation](docs/ARCHITECTURE.md) for details.

## Contributing

Contributions are welcome! Please read our contributing guidelines first.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [OpenAI Whisper](https://github.com/openai/whisper) - The underlying model
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) - C++ implementation
- [Groq](https://groq.com/) - Fast cloud inference
- [SuperWhisper](https://superwhisper.com/) - Inspiration

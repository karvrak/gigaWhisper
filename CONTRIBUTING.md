# Contributing to GigaWhisper

First off, thanks for taking the time to contribute!

## How Can I Contribute?

### Reporting Bugs

- Use the [Bug Report](https://github.com/YOUR_USERNAME/gigawhisper/issues/new?template=bug_report.yml) template
- Include your GigaWhisper version and Windows version
- Describe the steps to reproduce the issue
- Include relevant logs if available

### Suggesting Features

- Use the [Feature Request](https://github.com/YOUR_USERNAME/gigawhisper/issues/new?template=feature_request.yml) template
- Explain the problem you're trying to solve
- Describe your proposed solution

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run the linters and tests
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to your branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

## Development Setup

### Prerequisites

- **Node.js** 20+
- **pnpm** 9+
- **Rust** (latest stable)
- **Windows** (for full testing)

### Getting Started

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/gigawhisper.git
cd gigawhisper

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri:dev
```

### Project Structure

```
gigawhisper/
├── src/                 # React/TypeScript frontend
│   ├── components/      # UI components
│   ├── hooks/           # Custom React hooks
│   └── windows/         # Window-specific components
├── src-tauri/           # Rust backend
│   └── src/
│       ├── audio/       # Audio capture
│       ├── commands/    # Tauri IPC commands
│       ├── config/      # Settings management
│       ├── models/      # Whisper model management
│       ├── output/      # Text injection
│       ├── shortcuts/   # Global hotkeys
│       └── transcription/  # Transcription engines
└── docs/                # Documentation & ADRs
```

### Code Style

**TypeScript/React:**
- Run `pnpm lint` before committing
- Run `pnpm format` to format code with Prettier

**Rust:**
- Run `cargo fmt` to format code
- Run `cargo clippy` to check for issues

### Building

```bash
# Build frontend only
pnpm build

# Build full application
pnpm tauri:build
```

## Architecture Decisions

Major architectural decisions are documented as ADRs in the `docs/ADR/` folder. Please review them before making significant changes.

## Questions?

Feel free to open a [Discussion](https://github.com/YOUR_USERNAME/gigawhisper/discussions) for any questions!

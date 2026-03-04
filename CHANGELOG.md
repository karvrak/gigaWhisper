# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.7] - 2026-03-04

### Changed
- Version bump to 1.0.7

## [1.0.6] - 2026-03-04

### Added
- Premium licensing architecture and documentation
- Deepgram and OpenAI transcription providers
- LLM module for AI-powered features
- Premium components and hooks (useCredits, usePremium, useContexts)
- Theme system with customizable themes
- TypeScript type definitions

### Changed
- Extended settings with new provider configurations
- Enhanced config migration and secrets management
- Improved mouse hook handling for shortcuts
- Updated ProviderToggle and SettingsPanel components
- Applied rustfmt formatting to all Rust source files
- Updated tests to match component changes

## [1.0.5] - 2025-03-01

### Added
- Mouse button shortcuts support (Mouse4/Mouse5) via Windows low-level hook
- HotkeyInput component now captures mouse buttons in addition to keyboard shortcuts
- Welcome screen in onboarding wizard with branded UI

### Changed
- Refactored UI styles with new indigo/violet color palette
- Simplified Tailwind configuration
- Redesigned SettingsPanel layout
- Improved Onboarding flow (5 steps instead of 4)
- Updated ProviderToggle and ModelSelector components

### Fixed
- Updated tests for Onboarding and ProviderToggle components

## [1.0.4] - 2025-03-01

### Fixed
- Updated pnpm-lock.yaml and removed stale package-lock.json
- Resolved Clippy warnings across Rust codebase

### Changed
- Added ESLint 9 flat config (`eslint.config.js`)

## [1.0.3] - 2025-01-27

### Added
- SECURITY.md and CHANGELOG.md documentation
- Codecov integration for test coverage reporting
- Playwright E2E test infrastructure
- Comprehensive unit tests (142 tests, ~65% coverage)
- ADRs for code signing, config migration, crash reporting

### Fixed
- Path traversal vulnerability in history commands
- Audio normalization NaN panic
- Test selector ambiguity in SettingsPanel

### Changed
- Updated .gitignore for test artifacts

## [1.0.2] - 2025-01-26

### Added
- SHA256 checksum verification for model downloads
- Comprehensive test coverage (~65%+)
- CI coverage reporting with Codecov
- SECURITY.md documentation
- ADR for crash reporting (opt-in)
- ADR for Windows code signing
- ADR for config schema migration

### Fixed
- Path traversal vulnerability in history commands
- Panic on NaN in audio normalization
- Update endpoint mismatch (CPU/CUDA variants)
- Thread synchronization using channels instead of sleep
- Corrupted cache clearing

### Changed
- Log levels now conditional (debug in dev, warn in production)
- Improved idle model unloading

### Security
- Input validation on all file path operations
- Secure API key storage using OS credential manager

## [1.0.1] - 2025-01-25

### Fixed
- Bundle CUDA DLLs with installer
- Clear corrupted cache on startup

## [1.0.0] - 2025-01-24

### Added
- Initial release
- Local Whisper transcription (tiny, base, small, medium, large models)
- Groq cloud transcription integration
- Push-to-talk recording mode
- Toggle recording mode
- Global keyboard shortcuts (customizable)
- System tray integration
- Auto-paste transcription to active window
- Transcription history with search
- Auto-updates via GitHub Releases
- CUDA support for GPU acceleration
- Voice Activity Detection (VAD)
- Onboarding wizard for first-time setup
- Dark/Light theme support

## [0.3.0] - 2025-01-18

### Added
- Variant-aware auto-update system (CPU/Vulkan/CUDA)
- Auto-update system using tauri-plugin-updater
- GPU acceleration support (Vulkan/CUDA)

### Fixed
- CUDA compilation fixes and performance optimizations
- Force x86-64-v2 CPU target for compatibility
- CUDA build fixes (cublas packages, visual_studio_integration)

[Unreleased]: https://github.com/karvrak/gigaWhisper/compare/v1.0.6...HEAD
[1.0.6]: https://github.com/karvrak/gigaWhisper/compare/v1.0.5...v1.0.6
[1.0.5]: https://github.com/karvrak/gigaWhisper/compare/v1.0.4...v1.0.5
[1.0.4]: https://github.com/karvrak/gigaWhisper/compare/v1.0.3...v1.0.4
[1.0.3]: https://github.com/karvrak/gigaWhisper/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/karvrak/gigaWhisper/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/karvrak/gigaWhisper/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/karvrak/gigaWhisper/compare/v0.3.0...v1.0.0
[0.3.0]: https://github.com/karvrak/gigaWhisper/releases/tag/v0.3.0

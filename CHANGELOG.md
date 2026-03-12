# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.16] - 2026-03-12

### Changed
- Version bump to 1.0.16

## [1.0.15] - 2026-03-09

### Added
- **Temporary context overrides** — apply context presets temporarily for a single recording session
- **Auto-switch context at recording start** — automatically switch to the appropriate context when recording begins

### Fixed
- Sync API key flags between settings and transcription providers

## [1.0.14] - 2026-03-09

### Added
- **Single executable build** — streamlined distribution with single .exe output
- **Context editor enhancements** — improved UI for managing context presets

### Changed
- Improved transcription service reliability and error handling

## [1.0.13] - 2026-03-09

### Added
- **Whisper model preloading** — model is loaded at startup for faster first transcription
- Auto-start and auto-update enabled by default for new installations

### Fixed
- License API URL updated to gigawhisper.com
- CI build fixes
- Cargo fmt formatting in premium module
- Premium state event emission on license activation/deactivation

## [1.0.11] - 2026-03-07

### Added
- **Context presets** — predefined contexts for different transcription scenarios
- **Post-processing UI** — new interface for configuring text post-processing rules
- Updated signing public key for auto-updates

### Fixed
- Updater artifact generation with `createUpdaterArtifacts`
- Replaced tauri-action with direct build and artifact verification
- NSIS updater bundle signature format (`.nsis.zip.sig`)
- Release workflow made idempotent on re-runs

## [1.0.10] - 2026-03-07

### Fixed
- **Auto-update broken: "Invalid encoding in minisign data"** — release workflow was looking for `.exe.sig` but Tauri v2 generates `.nsis.zip.sig` for the updater bundle, resulting in empty signatures in update manifests

## [1.0.9] - 2026-03-07

### Fixed
- **Critical: API keys not persisting** — keyring crate was compiled without Windows backend (`windows-native` feature), causing all API keys (Deepgram, Groq, OpenAI) to be stored in memory only and lost on restart
- **Audio file leak** — orphaned `.wav` files in AppData were never cleaned up when history entries were evicted (100-entry limit)

### Added
- Automatic cleanup of orphaned audio files on startup (logs freed space)
- Audio file deletion when history entries are evicted by the 100-entry limit
- Diagnostic logging for credential store errors (shows exact keyring failure reason)

### Changed
- Cloud transcription providers (Deepgram, Groq, OpenAI) now read API key once at construction instead of on every transcription request

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

[Unreleased]: https://github.com/karvrak/gigaWhisper/compare/v1.0.16...HEAD
[1.0.16]: https://github.com/karvrak/gigaWhisper/compare/v1.0.15...v1.0.16
[1.0.15]: https://github.com/karvrak/gigaWhisper/compare/v1.0.14...v1.0.15
[1.0.14]: https://github.com/karvrak/gigaWhisper/compare/v1.0.13...v1.0.14
[1.0.13]: https://github.com/karvrak/gigaWhisper/compare/v1.0.11...v1.0.13
[1.0.11]: https://github.com/karvrak/gigaWhisper/compare/v1.0.10...v1.0.11
[1.0.10]: https://github.com/karvrak/gigaWhisper/compare/v1.0.9...v1.0.10
[1.0.9]: https://github.com/karvrak/gigaWhisper/compare/v1.0.7...v1.0.9
[1.0.7]: https://github.com/karvrak/gigaWhisper/compare/v1.0.6...v1.0.7
[1.0.6]: https://github.com/karvrak/gigaWhisper/compare/v1.0.5...v1.0.6
[1.0.5]: https://github.com/karvrak/gigaWhisper/compare/v1.0.4...v1.0.5
[1.0.4]: https://github.com/karvrak/gigaWhisper/compare/v1.0.3...v1.0.4
[1.0.3]: https://github.com/karvrak/gigaWhisper/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/karvrak/gigaWhisper/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/karvrak/gigaWhisper/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/karvrak/gigaWhisper/compare/v0.3.0...v1.0.0
[0.3.0]: https://github.com/karvrak/gigaWhisper/releases/tag/v0.3.0

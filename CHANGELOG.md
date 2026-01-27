# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

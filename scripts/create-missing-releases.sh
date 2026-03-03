#!/bin/bash
# =============================================================================
# Create Missing GitHub Releases
# =============================================================================
# This script creates GitHub releases for tags that don't have one yet.
# Requires: gh CLI (https://cli.github.com/)
#
# Usage: ./scripts/create-missing-releases.sh
# =============================================================================

set -e

if ! command -v gh &>/dev/null; then
  echo "Error: gh CLI not found. Install it from https://cli.github.com/"
  exit 1
fi

REPO="karvrak/gigaWhisper"

echo "=== Creating missing GitHub releases for $REPO ==="
echo ""

# v1.0.0 and v1.0.1 - historical releases without binaries
gh release create v1.0.0 \
  --repo "$REPO" \
  --title "GigaWhisper v1.0.0" \
  --notes "$(cat <<'EOF'
## Initial Release

### Added
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

> **Note:** This is a historical release. No pre-built binaries are available for this version. Please use [the latest release](https://github.com/karvrak/gigaWhisper/releases/latest) instead.
EOF
)" || echo "v1.0.0 release already exists, skipping."

echo ""

gh release create v1.0.1 \
  --repo "$REPO" \
  --title "GigaWhisper v1.0.1" \
  --notes "$(cat <<'EOF'
### Fixed
- Bundle CUDA DLLs with installer
- Clear corrupted cache on startup

> **Note:** This is a historical release. No pre-built binaries are available for this version. Please use [the latest release](https://github.com/karvrak/gigaWhisper/releases/latest) instead.
EOF
)" || echo "v1.0.1 release already exists, skipping."

echo ""

gh release create v1.0.3 \
  --repo "$REPO" \
  --title "GigaWhisper v1.0.3" \
  --notes "$(cat <<'EOF'
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

> **Note:** This is a historical release. No pre-built binaries are available for this version. Please use [the latest release](https://github.com/karvrak/gigaWhisper/releases/latest) instead.
EOF
)" || echo "v1.0.3 release already exists, skipping."

echo ""

gh release create v1.0.5 \
  --repo "$REPO" \
  --title "GigaWhisper v1.0.5" \
  --notes "$(cat <<'EOF'
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

---

## Installation

Download the installer for your system:

| Variant | Description | Download |
|---------|-------------|----------|
| **CPU** | Works on all systems | `GigaWhisper_1.0.5_x64-cpu-setup.exe` |
| **Vulkan** | GPU acceleration (AMD/Intel/NVIDIA) | `GigaWhisper_1.0.5_x64-vulkan-setup.exe` |
| **CUDA** | Best for NVIDIA GPUs | `GigaWhisper_1.0.5_x64-cuda-setup.exe` |

> **Note:** Binaries for this version may need to be rebuilt. If no .exe files are attached, use `git push origin v1.0.5` to re-trigger the build workflow, or use [the latest release](https://github.com/karvrak/gigaWhisper/releases/latest).
EOF
)" || echo "v1.0.5 release already exists, skipping."

echo ""
echo "=== Done! Check releases at https://github.com/$REPO/releases ==="

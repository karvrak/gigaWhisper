#!/bin/bash
# =============================================================================
# GigaWhisper Release Script
# =============================================================================
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 1.0.6
#
# This script:
# 1. Bumps version in all 3 config files (package.json, Cargo.toml, tauri.conf.json)
# 2. Prompts you to update CHANGELOG.md
# 3. Commits the version bump
# 4. Creates a git tag
# 5. Pushes tag to trigger the release workflow
# =============================================================================

set -e

VERSION="$1"

if [ -z "$VERSION" ]; then
  echo "Usage: ./scripts/release.sh <version>"
  echo "Example: ./scripts/release.sh 1.0.6"
  exit 1
fi

# Validate version format
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: Version must be in semver format (e.g., 1.0.6)"
  exit 1
fi

TAG="v$VERSION"

echo "=== GigaWhisper Release $TAG ==="
echo ""

# Check for clean working directory
if [ -n "$(git status --porcelain)" ]; then
  echo "Error: Working directory is not clean. Commit or stash your changes first."
  git status --short
  exit 1
fi

# Check that we're on main
BRANCH=$(git branch --show-current)
if [ "$BRANCH" != "main" ]; then
  echo "Warning: You're on branch '$BRANCH', not 'main'."
  read -p "Continue anyway? (y/N) " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 1
  fi
fi

# Check that tag doesn't already exist
if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Error: Tag $TAG already exists."
  exit 1
fi

echo "1/5 Bumping version to $VERSION..."

# Update package.json
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" package.json

# Update Cargo.toml (only the first version line in [package])
sed -i "0,/^version = \"[^\"]*\"/s//version = \"$VERSION\"/" src-tauri/Cargo.toml

# Update tauri.conf.json
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json

# Update Cargo.lock
echo "2/5 Updating Cargo.lock..."
(cd src-tauri && cargo update -p gigawhisper 2>/dev/null || cargo generate-lockfile 2>/dev/null || true)

echo ""
echo "3/5 CHANGELOG check..."
echo "---"
# Check if CHANGELOG has an entry for this version
if grep -q "\[$VERSION\]" CHANGELOG.md; then
  echo "CHANGELOG.md already has an entry for $VERSION."
else
  echo "WARNING: No entry for $VERSION found in CHANGELOG.md!"
  echo "Please update CHANGELOG.md before continuing."
  echo ""
  echo "Steps:"
  echo "  1. Move items from [Unreleased] to [$VERSION] - $(date +%Y-%m-%d)"
  echo "  2. Add comparison link at bottom of file"
  echo "  3. Save the file"
  echo ""
  read -p "Open CHANGELOG.md in editor? (Y/n) " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    ${EDITOR:-code} CHANGELOG.md
    echo "Press Enter when done editing CHANGELOG.md..."
    read
  fi
fi

echo "4/5 Committing and tagging..."
git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "chore: bump version to $VERSION"
git tag -a "$TAG" -m "Release $TAG"

echo ""
echo "5/5 Ready to push!"
echo ""
echo "Review the commit:"
git log --oneline -1
echo ""
echo "Tag: $TAG"
echo ""
read -p "Push commit and tag to trigger release? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  git push origin main
  git push origin "$TAG"
  echo ""
  echo "Done! Release workflow triggered."
  echo "Watch progress: https://github.com/karvrak/gigaWhisper/actions"
else
  echo ""
  echo "Not pushed. Run manually when ready:"
  echo "  git push origin main && git push origin $TAG"
fi

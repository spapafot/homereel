#!/usr/bin/env bash
# Downloads the pinned macOS ffmpeg build and installs it as the Tauri sidecar.
#
# Fetches the Apple Silicon release build from ffmpeg.martin-riedl.de, verifies its SHA-256, and
# copies ffmpeg to src-tauri/binaries/ffmpeg-aarch64-apple-darwin (the target-triple name Tauri's
# externalBin expects). The binary is gitignored; FFMPEG_VERSION.macos.txt records what was
# installed. Windows uses scripts/fetch-ffmpeg.ps1.
#
# Usage: bash scripts/fetch-ffmpeg.sh [--force]

set -euo pipefail

VERSION='9.0.2'
URL="https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_${VERSION}/ffmpeg.zip"
EXPECTED_SHA256='c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924'

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARIES_DIR="$REPO_ROOT/src-tauri/binaries"
TARGET="$BINARIES_DIR/ffmpeg-aarch64-apple-darwin"
VERSION_FILE="$BINARIES_DIR/FFMPEG_VERSION.macos.txt"

FORCE=false
if [[ "${1:-}" == '--force' ]]; then
  FORCE=true
fi

if [[ -f "$TARGET" && "$FORCE" == false ]]; then
  echo "ffmpeg sidecar already present at $TARGET (use --force to replace)."
  exit 0
fi

# Download into a fresh, empty directory so nothing else is executed from the archive location.
WORK_DIR="$(mktemp -d -t homereel-ffmpeg)"
trap 'rm -rf "$WORK_DIR"' EXIT

ZIP_PATH="$WORK_DIR/ffmpeg.zip"
echo "Downloading $URL"
curl --fail --location --silent --show-error --output "$ZIP_PATH" "$URL"

ACTUAL_SHA256="$(shasum -a 256 "$ZIP_PATH" | cut -d ' ' -f 1)"
if [[ "$ACTUAL_SHA256" != "$EXPECTED_SHA256" ]]; then
  echo "SHA-256 mismatch for ffmpeg.zip. Expected $EXPECTED_SHA256, got $ACTUAL_SHA256." >&2
  exit 1
fi
echo 'SHA-256 verified.'

EXTRACT_DIR="$WORK_DIR/extracted"
unzip -q "$ZIP_PATH" -d "$EXTRACT_DIR"
FFMPEG_BINARY="$(find "$EXTRACT_DIR" -type f -name ffmpeg | head -n 1)"
if [[ -z "$FFMPEG_BINARY" ]]; then
  echo 'ffmpeg not found inside ffmpeg.zip.' >&2
  exit 1
fi

mkdir -p "$BINARIES_DIR"
cp "$FFMPEG_BINARY" "$TARGET"
chmod +x "$TARGET"
BINARY_SHA256="$(shasum -a 256 "$TARGET" | cut -d ' ' -f 1)"
cat > "$VERSION_FILE" <<EOF
ffmpeg $VERSION (martin-riedl.de macOS arm64 release build, GPLv3)
source: $URL
archive sha256: $EXPECTED_SHA256
ffmpeg sha256: $BINARY_SHA256
EOF

echo "Installed ffmpeg $VERSION sidecar at $TARGET"

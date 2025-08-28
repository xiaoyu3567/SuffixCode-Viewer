#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$SCRIPT_DIR/egui_txt_viewer"

cd "$APP_DIR"
echo "Building (release)..."
cargo build --release

echo "Preparing app icon overlay (macOS dock) if available..."
if command -v sips >/dev/null 2>&1 && [[ -f "$SCRIPT_DIR/S.png" ]]; then
  # Note: Native app bundling is not implemented here; showing how to set dock icon for current process is non-trivial in pure bash.
  # Placeholder: no-op. For full app bundle with icon, consider cargo-bundle or create-dmg later.
  :
fi

echo "Running app..."
"$APP_DIR/target/release/egui_txt_viewer" "$@"


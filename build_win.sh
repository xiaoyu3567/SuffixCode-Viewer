#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$SCRIPT_DIR/egui_txt_viewer"

echo "==> Ensuring Windows target installed (x86_64-pc-windows-gnu)"
rustup target add x86_64-pc-windows-gnu || true

if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
  echo "==> Installing mingw-w64 via Homebrew (requires brew)"
  if ! command -v brew >/dev/null 2>&1; then
    echo "Homebrew not found. Please install Homebrew first: https://brew.sh" >&2
    exit 1
  fi
  brew install mingw-w64
fi

export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++

cd "$APP_DIR"
# Prepare icon (.ico) from root S.png if available (avoid spaces in path)
ICO_TMP_DIR="$APP_DIR/.icons_win"
mkdir -p "$ICO_TMP_DIR"
if [[ -f "$SCRIPT_DIR/S.png" ]]; then
  echo "==> Preparing Windows icon from S.png"
  if command -v magick >/dev/null 2>&1; then
    magick "$SCRIPT_DIR/S.png" -define icon:auto-resize=256,128,64,48,32,16 "$ICO_TMP_DIR/app.ico"
  else
    echo "ImageMagick not found (magick). Install via: brew install imagemagick" >&2
  fi
fi

# If .ico exists, compile resource and pass to linker
if [[ -f "$ICO_TMP_DIR/app.ico" ]]; then
  echo 'IDI_ICON1 ICON "app.ico"' > "$ICO_TMP_DIR/app.rc"
  (cd "$ICO_TMP_DIR" && x86_64-w64-mingw32-windres app.rc -O coff -o app_res.o)
  # Use a relative path (no spaces) from APP_DIR to avoid rustc parsing issues
  export RUSTFLAGS="-Clink-arg=.icons_win/app_res.o"
fi

echo "==> Building Windows release (.exe)"
cargo build --release --target x86_64-pc-windows-gnu

OUT="$APP_DIR/target/x86_64-pc-windows-gnu/release/egui_txt_viewer.exe"
DEST="$APP_DIR/target/x86_64-pc-windows-gnu/release/SuffixCode Viewer V0.1.exe"
if [[ -f "$OUT" ]]; then
  mv -f "$OUT" "$DEST"
  echo "==> Built: $DEST"
else
  echo "Build failed: $OUT not found" >&2
  exit 1
fi


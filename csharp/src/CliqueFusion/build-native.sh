#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_ROOT=$(cargo locate-project --workspace --message-format plain | xargs dirname)

PROJECT_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

echo "Building native FFI for current platform..."

OS="$(uname -s)"
case "$OS" in
  Linux*)
    RID="linux-x64"
    LIB="libclique_fusion_ffi.so"
    ;;
  Darwin*)
    RID="osx-x64"
    LIB="libclique_fusion_ffi.dylib"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    RID="win-x64"
    LIB="clique_fusion_ffi.dll"
    ;;
  *)
    echo "Unsupported OS for native build: $OS" >&2
    exit 1
    ;;
esac

echo "→ Building $RID"
cargo build --release --package clique-fusion-ffi --manifest-path "$WORKSPACE_ROOT/Cargo.toml"

SRC="$WORKSPACE_ROOT/target/release/$LIB"
DEST="$PROJECT_ROOT/runtimes/$RID/native"
DEST_LIB="$DEST/$LIB"

mkdir -p "$DEST"
if [[ -f "$DEST_LIB" ]] && cmp -s "$SRC" "$DEST_LIB"; then
  echo "✔ $LIB unchanged"
else
  echo "→ Updating $DEST_LIB"
  cp "$SRC" "$DEST"
fi

echo "✅ Done."

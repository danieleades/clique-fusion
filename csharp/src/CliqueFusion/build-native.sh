#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_ROOT=$(cargo locate-project --workspace --message-format plain | xargs dirname)

echo "Checking native FFI builds..."

TARGETS=(
  "linux-x64 x86_64-unknown-linux-gnu libclique_fusion_ffi.so"
  "win-x64   x86_64-pc-windows-gnu   clique_fusion_ffi.dll"
)

for entry in "${TARGETS[@]}"; do
  read -r RID TARGET LIB <<< "$entry"

  echo "→ Building $RID ($TARGET)"
  cross build --release --target "$TARGET" --package clique-fusion-ffi

  SRC="$WORKSPACE_ROOT/target/$TARGET/release/$LIB"
  DEST="runtimes/$RID/native"
  DEST_LIB="$DEST/$LIB"

  # Copy only if the file changed
  mkdir -p "$DEST"
  if cmp -s "$SRC" "$DEST_LIB"; then
    echo "✔ $LIB unchanged for $RID"
  else
    echo "→ Updating $DEST_LIB"
    cp "$SRC" "$DEST"
  fi
done

echo "✅ Done."

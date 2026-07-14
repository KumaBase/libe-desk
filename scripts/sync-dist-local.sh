#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/dist-local"

CANDIDATES=(
  "$ROOT/src-tauri/target/release/bundle/macos/Libe Desk.app"
)
while IFS= read -r p; do
  CANDIDATES+=("$p")
done < <(find /var/folders -path '*/cargo-target/release/bundle/macos/Libe Desk.app' 2>/dev/null | head -20)

NEWEST=""
NEWEST_MTIME=0
for app in "${CANDIDATES[@]}"; do
  bin="$app/Contents/MacOS/libe-desk"
  [[ -x "$bin" ]] || continue
  mtime=$(stat -f %m "$bin")
  if (( mtime > NEWEST_MTIME )); then
    NEWEST_MTIME=$mtime
    NEWEST="$app"
  fi
done

if [[ -z "$NEWEST" ]]; then
  echo "Libe Desk.app not found. Run: npm run tauri build" >&2
  exit 1
fi

mkdir -p "$DEST"
rm -rf "$DEST/Libe Desk.app"
ditto "$NEWEST" "$DEST/Libe Desk.app"
echo "Synced from: $NEWEST"
ls -la "$DEST/Libe Desk.app/Contents/MacOS/libe-desk"

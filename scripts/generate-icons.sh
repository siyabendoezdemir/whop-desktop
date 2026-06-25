#!/usr/bin/env bash
#
# Regenerate ALL Tauri/macOS icon sizes from a single 1024x1024 PNG.
#
# Usage:
#   ./scripts/generate-icons.sh [path/to/icon-1024.png]
#
# If no path is given, it (re)creates the neutral placeholder first.
# This wraps `pnpm tauri icon`, which writes into src-tauri/icons/.
set -euo pipefail

cd "$(dirname "$0")/.."

SRC="${1:-}"

if [[ -z "$SRC" ]]; then
  echo "No source PNG given — generating neutral placeholder..."
  SRC="src-tauri/app-icon-source.png"
  python3 scripts/make-placeholder-icon.py "$SRC"
fi

if [[ ! -f "$SRC" ]]; then
  echo "error: source icon '$SRC' not found" >&2
  exit 1
fi

echo "Generating icons from: $SRC"
pnpm tauri icon "$SRC"

echo "Done. Icons written to src-tauri/icons/"

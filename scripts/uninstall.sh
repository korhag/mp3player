#!/usr/bin/env bash
# ------------------------------------------------------------
# Script: uninstall.sh
# Purpose: Removes the per-user Linux install and desktop entry.
#          Asks before deleting playlists and settings.
#
# Usage:
#   ./scripts/uninstall.sh
#   ./scripts/uninstall.sh --no-prompt
#   ./scripts/uninstall.sh --delete-data
# ------------------------------------------------------------

set -euo pipefail

NO_PROMPT=0
DELETE_DATA=0

for arg in "$@"; do
    case "$arg" in
        --no-prompt) NO_PROMPT=1 ;;
        --delete-data) DELETE_DATA=1 ;;
        *)
            echo "Unknown option: $arg" >&2
            exit 1
            ;;
    esac
done

BIN="$HOME/.local/bin/soundkeep"
DESKTOP="$HOME/.local/share/applications/soundkeep.desktop"
ICON="$HOME/.local/share/icons/hicolor/256x256/apps/soundkeep.png"

DATA_DIR=""
if [[ -x "$BIN" ]]; then
    DATA_DIR="$("$BIN" --print-data-dir 2>/dev/null || true)"
fi
if [[ -z "$DATA_DIR" ]]; then
    if [[ -n "${XDG_DATA_HOME:-}" ]]; then
        DATA_DIR="$XDG_DATA_HOME/soundkeep"
    else
        DATA_DIR="$HOME/.local/share/soundkeep"
    fi
fi

if [[ -d "$DATA_DIR" && "$DELETE_DATA" -eq 0 && "$NO_PROMPT" -eq 0 ]]; then
    read -r -p "Delete playlists and settings in $DATA_DIR ? [y/N] " answer || true
    if [[ "$answer" =~ ^[yY] ]]; then
        DELETE_DATA=1
    fi
fi

rm -f "$BIN" "$DESKTOP" "$ICON"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

if [[ "$DELETE_DATA" -eq 1 && -d "$DATA_DIR" ]]; then
    rm -rf "$DATA_DIR"
    echo "Deleted playlists and settings: $DATA_DIR"
elif [[ -d "$DATA_DIR" ]]; then
    echo "Kept playlists and settings: $DATA_DIR"
fi

echo "Soundkeep was removed."

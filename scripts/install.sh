#!/usr/bin/env bash
# ------------------------------------------------------------
# Script: install.sh
# Purpose: Installs build dependencies, builds the release
#          player, and registers it for the current Linux user.
#          No root is required for the app itself. Package
#          installs still use sudo, and only after a prompt.
#
# Usage:
#   ./scripts/install.sh
#   ./scripts/install.sh --no-prompt
#   ./scripts/install.sh --default-app
#
# Re-run the script to upgrade an existing install.
# Keep the MIME list in sync with EXTENSIONS in src/config.rs.
# ------------------------------------------------------------

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NO_PROMPT=0
SET_DEFAULT=0

for arg in "$@"; do
    case "$arg" in
        --no-prompt) NO_PROMPT=1 ;;
        --default-app) SET_DEFAULT=1 ;;
        *)
            echo "Unknown option: $arg" >&2
            exit 1
            ;;
    esac
done

ask_yes() {
    local prompt="$1"
    if [[ "$NO_PROMPT" -eq 1 ]]; then
        return 1
    fi
    local answer
    read -r -p "$prompt [y/N] " answer || true
    [[ "$answer" =~ ^[yY] ]]
}

need_pkg_config() {
    if ! command -v pkg-config >/dev/null 2>&1; then
        return 0
    fi
    pkg-config --exists alsa \
        && pkg-config --exists xkbcommon \
        && pkg-config --exists fontconfig \
        || return 0
    return 1
}

install_packages() {
    local packages=("$@")
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y "${packages[@]}"
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y "${packages[@]}"
    elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -S --needed --noconfirm "${packages[@]}"
    elif command -v zypper >/dev/null 2>&1; then
        sudo zypper install -y "${packages[@]}"
    else
        echo "Could not detect apt, dnf, pacman, or zypper." >&2
        echo "Install these packages yourself, then rerun: ${packages[*]}" >&2
        exit 1
    fi
}

echo "Soundkeep install"
echo "Project: $ROOT"

if need_pkg_config; then
    packages=()
    if command -v apt-get >/dev/null 2>&1; then
        packages=(pkg-config libasound2-dev libxkbcommon-dev libwayland-dev libfontconfig1-dev build-essential)
    elif command -v dnf >/dev/null 2>&1; then
        packages=(pkgconf-pkg-config alsa-lib-devel wayland-devel libxkbcommon-devel fontconfig-devel gcc)
    elif command -v pacman >/dev/null 2>&1; then
        packages=(pkgconf alsa-lib wayland libxkbcommon fontconfig base-devel)
    elif command -v zypper >/dev/null 2>&1; then
        packages=(pkg-config alsa-devel wayland-devel libxkbcommon-devel fontconfig-devel gcc)
    fi
    echo "Missing build packages: ${packages[*]}"
    if [[ "$NO_PROMPT" -eq 1 ]]; then
        echo "Re-run without --no-prompt to install them, or install them yourself." >&2
        exit 1
    fi
    if ask_yes "Install those packages with sudo?"; then
        install_packages "${packages[@]}"
    else
        echo "Build dependencies are required. Stopped." >&2
        exit 1
    fi
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust (cargo) was not found."
    if [[ "$NO_PROMPT" -eq 1 ]] || ! ask_yes "Install Rust with rustup?"; then
        echo "Install Rust from https://rustup.rs and run this script again." >&2
        exit 1
    fi
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
fi

echo "Building release binary..."
(cd "$ROOT" && cargo build --release)

mkdir -p \
    "$HOME/.local/bin" \
    "$HOME/.local/share/applications" \
    "$HOME/.local/share/icons/hicolor/256x256/apps"

install -m 755 "$ROOT/target/release/soundkeep" "$HOME/.local/bin/soundkeep"
install -m 644 "$ROOT/assets/icon.png" "$HOME/.local/share/icons/hicolor/256x256/apps/soundkeep.png"
install -m 644 "$ROOT/packaging/linux/soundkeep.desktop" "$HOME/.local/share/applications/soundkeep.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" || true
fi
# A failed rebuild is not an install error. GTK still finds the icon
# by scanning the folder. The shared hicolor theme often cannot be
# cached because other apps install icons whose names contain spaces.
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *)
        echo "Add this to your shell profile if it is not already there:"
        echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
        ;;
esac

if [[ "$SET_DEFAULT" -eq 0 ]] && [[ "$NO_PROMPT" -eq 0 ]]; then
    if ask_yes "Make Soundkeep the default player for these audio types?"; then
        SET_DEFAULT=1
    fi
fi

if [[ "$SET_DEFAULT" -eq 1 ]] && command -v xdg-mime >/dev/null 2>&1; then
    mime_types=(
        audio/mpeg audio/mp3 audio/flac audio/x-flac audio/ogg audio/vorbis
        audio/x-vorbis+ogg audio/opus audio/x-opus+ogg audio/wav audio/x-wav
        audio/aac audio/mp4 audio/x-m4a audio/aiff audio/x-aiff
    )
    for mime in "${mime_types[@]}"; do
        xdg-mime default soundkeep.desktop "$mime" || true
    done
fi

echo
echo "Installed Soundkeep to $HOME/.local/bin/soundkeep"
echo "Open it from the application menu, or right-click an audio file and choose Open with."
echo "To remove it, run ./scripts/uninstall.sh"

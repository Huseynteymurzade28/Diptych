#!/usr/bin/env bash
# Installs Diptych for the current user (or system-wide with --prefix /usr/local).
#
#   scripts/install.sh              build and install to ~/.local
#   scripts/install.sh --default    …and make it the default file manager
#   scripts/install.sh --uninstall  remove what was installed
#
# Options: --prefix DIR (default ~/.local), --no-build (use target/release).
set -euo pipefail

cd "$(dirname "$0")/.."
PREFIX="$HOME/.local"
DEFAULT=0
UNINSTALL=0
BUILD=1
while [ $# -gt 0 ]; do
    case "$1" in
        --prefix) PREFIX="$2"; shift ;;
        --default) DEFAULT=1 ;;
        --uninstall) UNINSTALL=1 ;;
        --no-build) BUILD=0 ;;
        -h|--help) sed -n '2,8p' "$0"; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
    shift
done

ID=com.flear.diptych
BIN="$PREFIX/bin/diptych"
APPS="$PREFIX/share/applications"
ICONS="$PREFIX/share/icons/hicolor/scalable/apps"
SERVICES="$PREFIX/share/dbus-1/services"

refresh_caches() {
    command -v update-desktop-database >/dev/null && update-desktop-database -q "$APPS" || true
    command -v gtk-update-icon-cache >/dev/null &&
        gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
}

if [ "$UNINSTALL" = 1 ]; then
    rm -fv "$BIN" "$APPS/$ID.desktop" "$ICONS/$ID.svg" "$SERVICES/$ID.FileManager1.service"
    refresh_caches
    echo "If Diptych was your default file manager, pick another one, e.g.:"
    echo "  xdg-mime default org.kde.dolphin.desktop inode/directory"
    exit 0
fi

if [ "$BUILD" = 1 ]; then
    cargo build --release
fi
install -Dm755 target/release/diptych "$BIN"
# Absolute Exec path, so it works even if $PREFIX/bin isn't in $PATH.
install -d "$APPS"
sed "s|^Exec=diptych|Exec=$BIN|" "data/$ID.desktop" > "$APPS/$ID.desktop"
install -Dm644 "data/icons/$ID.svg" "$ICONS/$ID.svg"
refresh_caches
echo "Installed $BIN"

if [ "$DEFAULT" = 1 ]; then
    xdg-mime default "$ID.desktop" inode/directory
    # "Show in folder" from browsers and other apps, even when Diptych
    # isn't running.
    install -d "$SERVICES"
    sed "s|@BINDIR@|$PREFIX/bin|" "data/$ID.FileManager1.service" \
        > "$SERVICES/$ID.FileManager1.service"
    echo "Diptych is now the default file manager ($(xdg-mime query default inode/directory))."
fi

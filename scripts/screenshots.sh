#!/usr/bin/env bash
# Renders Diptych the way it looks on each desktop and fails if GTK logs
# a critical or a warning from Diptych's own widgets. CI runs it under
# headless Weston; locally it uses your current Wayland session.
#
#   scripts/screenshots.sh [BINARY]        (default target/debug/diptych)
#
# Environment: OUT (default screenshots/), WESTON=1 to start a headless
# Weston first.
set -euo pipefail

cd "$(dirname "$0")/.."
BIN=$(realpath "${1:-target/debug/diptych}")
OUT=$(realpath -m "${OUT:-screenshots}")
mkdir -p "$OUT"
WORK=$(mktemp -d)
WESTON_PID=
cleanup() {
    [ -n "$WESTON_PID" ] && kill "$WESTON_PID" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

if [ "${WESTON:-0}" = 1 ]; then
    export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$WORK/runtime}"
    mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
    weston --backend=headless --socket=diptych-shots --width=1280 --height=800 \
        > "$OUT/weston.log" 2>&1 &
    WESTON_PID=$!
    for _ in $(seq 50); do
        [ -S "$XDG_RUNTIME_DIR/diptych-shots" ] && break
        sleep 0.1
    done
    [ -S "$XDG_RUNTIME_DIR/diptych-shots" ] || { cat "$OUT/weston.log"; exit 1; }
    export WAYLAND_DISPLAY=diptych-shots
fi

# name  XDG_CURRENT_DESKTOP  extra theme.toml
SHOTS=(
    "gnome|GNOME|"
    "kde|KDE|"
    "hyprland|Hyprland|"
    "hyprland-translucent|Hyprland|[colors]\nwindow = \"#1e1e2ecc\"\nsidebar = \"#18182540\""
    "light|GNOME|base = \"cozy-latte\""
)

failed=0
for shot in "${SHOTS[@]}"; do
    IFS='|' read -r name desktop theme <<< "$shot"
    home="$WORK/$name"
    mkdir -p "$home/config/diptych" "$home/Documents/Reports" "$home/Pictures" "$home/Music"
    printf '# Notes\n' > "$home/notes.md"
    printf 'fn main() {}\n' > "$home/main.rs"
    printf 'a,b\n1,2\n' > "$home/Documents/budget.csv"
    : > "$home/Documents/report.pdf"
    [ -n "$theme" ] && printf "$theme\n" > "$home/config/diptych/theme.toml"

    log="$OUT/$name.log"
    env HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_DATA_HOME="$home/data" \
        XDG_CACHE_HOME="$home/cache" XDG_CURRENT_DESKTOP="$desktop" \
        GTK_A11Y=none DIPTYCH_SNAPSHOT="$OUT/$name.png" DIPTYCH_SNAPSHOT_DELAY_MS=2500 \
        timeout 60 "$BIN" > "$log" 2>&1 || true

    if [ ! -s "$OUT/$name.png" ]; then
        echo "✗ $name: no screenshot"; failed=1
    elif grep -qE -- '-CRITICAL|Diptych-WARNING|Gtk-WARNING|Adwaita-WARNING' "$log"; then
        echo "✗ $name: GTK reported problems"; failed=1
    else
        echo "✓ $name ($desktop)"
        continue
    fi
    sed 's/^/    /' "$log"
done
exit $failed

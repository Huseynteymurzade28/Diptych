# Desktops

Diptych is a GTK4 and libadwaita app. It aims to feel native and stay customizable on GNOME, KDE Plasma
and tiling Wayland compositors such as Hyprland, Sway and niri. Progress is tracked in #17.

## Window decorations

With `decorations = "auto"` in [`layout.toml`](layout.md) (the default), Diptych reads `$XDG_CURRENT_DESKTOP`:

| Desktop | `auto` means | Why |
|---|---|---|
| GNOME | `full` | Standard header bar with window buttons |
| KDE Plasma | `full` | Button order follows Plasma's setting (*System Settings → Colors & Themes → Application Style → Configure GNOME/GTK Application Style*), which Plasma writes to `gtk-decoration-layout` |
| Hyprland, Sway, niri, river, i3, bspwm, qtile, dwl, wayfire, labwc | `minimal` | The compositor manages windows, so close/maximize buttons are dead weight |

You can force any mode. `none` removes the header bar entirely, for a fully keyboard-driven setup.

## Tiling and small windows

Tiles are often half or a quarter of the screen, so the layout adapts:

* Below `collapse-inspector-below` (900 px), the inspector becomes an overlay. Toggle it from the header.
* Below `collapse-sidebar-below` (600 px), the sidebar does too, and the view switcher moves into the main
  menu's **View** section.
* Long paths scroll inside the path bar instead of widening the window.
* The window can shrink to 360 × 300.

## System settings Diptych follows

| Setting | How |
|---|---|
| UI font | `fonts.ui = "system"` (default) uses your desktop font (`gtk-font-name`). On KDE this is the font Plasma exports to GTK apps. |
| Light / dark | With `base-light` set in `theme.toml` (or `mode = "system"`), Diptych switches presets when the desktop's preference changes. It uses the freedesktop settings portal. The dark preference was verified on KDE Plasma through the portal, and the light switch was verified with `ADW_DEBUG_COLOR_SCHEME=prefer-light`. It should also work on GNOME and on Hyprland with `xdg-desktop-portal-gtk` or `-hyprland`, but that hasn't been tested yet. |
| Icon theme | Your icon theme (Adwaita, Breeze, Papirus, Tela…). The inspector uses GIO's content-type icons, which fall back gracefully. |

## Installing and making Diptych the default

```sh
scripts/install.sh             # build, install to ~/.local (binary, .desktop, icon)
scripts/install.sh --default   # …and make it the default file manager
scripts/install.sh --uninstall
```

`--prefix /usr/local` installs system-wide. `--default` does two things:

* `xdg-mime default com.flear.diptych.desktop inode/directory`: "Open folder" from any app, `xdg-open ~/Downloads`
  and desktop icons open Diptych.
* It installs a D-Bus service for `org.freedesktop.FileManager1`. This is what browsers, download managers and
  IDEs call for **Show in folder**. Diptych opens the folder with the file selected, even if it wasn't running.

Diptych only answers `FileManager1` while it's the `inode/directory` default (or was started for it), so trying
it next to Dolphin or Nautilus doesn't take over their "Show in folder". To switch back, run e.g.
`xdg-mime default org.kde.dolphin.desktop inode/directory`.

From a terminal, `diptych ~/Downloads` opens a folder, and `diptych ~/Downloads/file.pdf` opens its folder with the
file selected.

## Open Terminal Here

`Shift+F4`, the empty-space menu, or the command palette opens a terminal in the current folder (or in the
selected folder). The terminal is picked in this order:

1. [`xdg-terminal-exec`](https://gitlab.freedesktop.org/terminal-wg/specifications), if installed
2. `$TERMINAL` (it may include arguments, e.g. `kitty --single-instance`)
3. the desktop's terminal: Ptyxis, Console or GNOME Terminal on GNOME; Konsole on KDE; xfce4-terminal on Xfce
4. the first one found of kitty, foot, Alacritty, WezTerm, Ghostty, Konsole, Ptyxis, Console, GNOME Terminal,
   xfce4-terminal, xterm

On Hyprland and Sway, set `$TERMINAL` or install `xdg-terminal-exec` to pick yours. Verified on KDE Plasma, where
Konsole opens with `--workdir`.

## Translucency

An alpha channel in the `window` color gives a translucent window, which Hyprland blurs. See
[theming.md](theming.md#translucency-and-blur) for how each desktop handles blur.

## Known issues

* **Tela icon theme with GTK 4.22.** Some Tela symbolic icons render blank, e.g. `list-add-symbolic` and
  `value-increase-symbolic` (the −/+ buttons of number fields). The affected SVGs wrap their path in
  `<g transform="translate(…)">`, and GTK 4.22 appears not to apply it. Diptych's *New* button uses
  `folder-new-symbolic`, which renders fine. Details are in #16.
* Missing icon names on stock Adwaita for some file kinds: #16.

## Screenshots in CI

Every push renders the window under headless Weston as GNOME, KDE and Hyprland users see it, plus a
translucent and a light theme (`scripts/screenshots.sh`). The job fails if GTK logs a critical or a
warning, and the PNGs are uploaded as the `screenshots` artifact. Run it locally with
`scripts/screenshots.sh` (it uses your session) or `WESTON=1 scripts/screenshots.sh`.

These are simulated desktops: only `XDG_CURRENT_DESKTOP` changes, so they check Diptych's own
per-desktop behavior (window buttons, theme, layout), not the real compositors.

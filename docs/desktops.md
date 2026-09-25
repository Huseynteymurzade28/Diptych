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

## Known issues

* **Tela icon theme with GTK 4.22.** Tela's `list-add-symbolic` renders blank. This was seen on KDE
  Plasma, and the root cause hasn't been identified yet (the SVG uses KDE's `ColorScheme-Text` styling
  plus a `translate()` transform). Diptych's *New* button uses `folder-new-symbolic`, which renders fine.
* Missing icon names on stock Adwaita for some file kinds: #16.

## Planned (#17)

* Translucent window and sidebar colors so Hyprland/KWin blur shows through
* `.desktop` file, `inode/directory` MIME type (become the default file manager), `org.freedesktop.FileManager1`
* Terminal action resolving `xdg-terminal-exec` → `$TERMINAL` → the desktop default
* CI screenshots under headless Weston for each desktop

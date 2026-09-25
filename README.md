# 🦀 Diptych File Manager

**Diptych** is a highly performant, customizable file manager built from scratch using **Rust** and **GTK4**.

The name "Diptych" refers to a work of art made of two hinged parts—reflecting our dual-pane/inspector-based design philosophy.

## 🚀 Features

- **Pure Rust & GTK4:** Blazing fast performance and memory safety.
- **Cross-Platform:** Runs on any Linux distro (Arch, Fedora, Ubuntu) and can be compiled for Windows/macOS.
- **NixOS-Ready:** Includes a `flake.nix` for instant dev environments.
- **Diptych UX:** A unique "Select & Inspect" workflow.

## 🎨 Customization

Everything visual is driven by theme tokens in `~/.config/diptych/theme.toml`,
which is created on first launch and **applied live whenever you save it**:

```toml
base = "nord"                 # a built-in preset or ~/.config/diptych/themes/<id>.toml

[colors]
accent = "#ff79c6"

[shape]
radius = 6                    # corner radius in px
density = "compact"           # compact | comfortable | spacious

[fonts]
ui = "Figtree, sans-serif"
scale = 1.1
```

For anything tokens don't cover, write plain GTK CSS in `~/.config/diptych/user.css`
(also hot-reloaded). See [docs/theming.md](docs/theming.md) for every token and CSS class.

The window itself is configured in `~/.config/diptych/layout.toml`: which panes are shown and where,
their widths, header bar items and their order, inspector fields, and window decorations
([docs/layout.md](docs/layout.md)). Diptych adapts to GNOME, KDE Plasma and tiling compositors like
Hyprland ([docs/desktops.md](docs/desktops.md)).

## 🛠️ Development Setup

You can develop Diptych on **any operating system**.

### Option A: NixOS / Nix Users (Recommended)

1. Clone the repo.
2. Allow direnv to load the environment:
   ```bash
   direnv allow
   ```
   _(Or run `nix develop` manually)_
3. Run: `cargo run`

### Option B: Other Linux Distros (Arch, Fedora, Ubuntu, etc.)

1. Install system dependencies (GTK4):
   - **Arch:** `sudo pacman -S gtk4 libadwaita base-devel`
   - **Ubuntu/Debian:** `sudo apt install libgtk-4-dev libadwaita-1-dev build-essential`
   - **Fedora:** `sudo dnf install gtk4-devel libadwaita-devel gcc`
2. Run the project:
   ```bash
   cargo run
   ```

## 🏗️ Architecture

- **`src/filesystem`**: Handles low-level IO, directory scanning, and file metadata.
- **`src/ui`**: Contains the GTK4 widget logic and event handling.
- **`flake.nix`**: Provides the system libraries (GTK4, GLib, Wayland) needed for compilation (Optional for non-Nix users).


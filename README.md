# 🦀 Diptych File Manager

**Diptych** is a highly performant, customizable file manager built from scratch using **Rust** and **GTK4**. Configured for a reproducible **NixOS** development environment.

The name "Diptych" refers to a work of art made of two hinged parts—reflecting our dual-pane/inspector-based design philosophy.

## 🚀 Features

- **Pure Rust & GTK4:** Blazing fast performance and memory safety.
- **NixOS-First:** Initialized with a robust `flake.nix` for hermetic development.
- **Diptych UX:** A unique "Select & Inspect" workflow (Coming soon!).
- **Minimalist:** Bloat-free by design.

## 🛠️ Development Setup

### Prerequisite

- **Nix** (with Flakes enabled)
- `direnv` (Recommended)

### Quick Start

1. Clone the repo.
2. Allow direnv to load the environment:

   ```bash
   direnv allow
   ```

   _(Or run `nix develop` manually)_

3. Run the project:
   ```bash
   cargo run
   ```

## 🏗️ Architecture

- **`src/filesystem`**: Handles low-level IO, directory scanning, and file metadata.
- **`src/ui`**: Contains the GTK4 widget logic and event handling.
- **`flake.nix`**: Provides the system libraries (GTK4, GLib, Wayland) needed for compilation.

## 🤝 Contributing

This is a mentorship project. We iterate fast and break things often.

---

_Built with ❤️ by Flear & Copilot (The Mentor)_

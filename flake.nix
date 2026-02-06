{
  description = "Diptych File Manager Dev Environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain with analyzer for VSCode
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config # Essential for finding system libs
          ];

          buildInputs = with pkgs; [
            rustToolchain
            gtk4
            glib
            cairo
            pango
            gdk-pixbuf
            wayland
            wayland-protocols
            adwaita-icon-theme
            hicolor-icon-theme
          ];

          # Essential environment variables for GTK
          shellHook = ''
            export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk4}/share/gsettings-schemas/${pkgs.gtk4.name}:$XDG_DATA_DIRS
            export XDG_DATA_DIRS=${pkgs.adwaita-icon-theme}/share:$XDG_DATA_DIRS
            echo "🦀 Diptych Dev Environment Ready (Rust + GTK4) 🚀"
            echo "Rust: $(rustc --version)"
          '';
        };
      }
    );
}

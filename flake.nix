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
          ];

          # Sometimes useful for catching library path issues
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [ pkgs.gtk4 pkgs.glib ]}";

          shellHook = ''
            echo "🦀 Diptych Dev Environment Ready (Rust + GTK4) 🚀"
            echo "Rust: $(rustc --version)"
          '';
        };
      }
    );
}

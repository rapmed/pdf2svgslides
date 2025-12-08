{
  description = "pdf2svgslides";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url  = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust-version = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rev = if (self ? shortRev) then self.shortRev else "dev";

        pkgNativeBuildInputs = [
          rust-version
          pkgs.pkg-config
        ];
        pkgBuildInputs = [
          pkgs.cairo
          pkgs.glib
          pkgs.poppler
        ];
      in
      with pkgs;
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = pkgNativeBuildInputs;
          buildInputs = pkgBuildInputs;
        };

        packages.default = pkgs.rustPlatform.buildRustPackage rec {
          pname = "pdf2svgslides";
          version = rev;
          src = pkgs.lib.cleanSource self;
          cargoLock = { lockFile = ./Cargo.lock; };
          strictDeps = true;

          nativeBuildInputs = pkgNativeBuildInputs;
          buildInputs = pkgBuildInputs;

          # Avoid /nix/store paths in the binary, so that they don't get mixed up with dependencies
          RUSTFLAGS = "--remap-path-prefix ${rust-version}=/rust";

          meta = with lib; {
            description = "Splits PDF pages into SVG files, and generates a JPEG thumbnail for each.";
            homepage = "https://github.com/abustany/pdf2svgslides";
            license = with licenses; [ gpl2 ];
          };
        };
      }
    );
}

{
  description = "pdf2svgslides";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-26.05";
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
        # poppler 26.06.0 crashes on macOS when rendering a page:
        # https://gitlab.freedesktop.org/poppler/poppler/-/work_items/1743
        # Fixed in 26.07.0. Remove this overlay once nixpkgs ships a fixed version.
        popplerOverlay = final: prev:
          let
            version = "26.08.0";
          in {
            poppler =
              if !(prev.lib.versionOlder prev.poppler.version version)
              then throw "nixpkgs ships poppler ${prev.poppler.version}, which is >= ${version}: remove popplerOverlay from flake.nix"
              else prev.poppler.overrideAttrs (old: {
                inherit version;
                src = prev.fetchurl {
                  url = "https://poppler.freedesktop.org/poppler-${version}.tar.xz";
                  hash = "sha256-3JBuaM6mmBCXBqxqo9LJ1FEvz8rELZC4r82khtG5q9A=";
                };
              });
          };
        overlays = [(import rust-overlay) popplerOverlay];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust-version = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust-version;
          rustc = rust-version;
        };
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

        packages.default = rustPlatform.buildRustPackage rec {
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

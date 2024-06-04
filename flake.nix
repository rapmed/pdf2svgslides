{
  description = "pdf2svgslides";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-23.11";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rev = if (self ? shortRev) then self.shortRev else "dev";

        pkgNativeBuildInputs = [
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

          nativeBuildInputs = pkgNativeBuildInputs;
          buildInputs = pkgBuildInputs;

          meta = with lib; {
            description = "Splits PDF pages into SVG files, and generates a JPEG thumbnail for each.";
            homepage = "https://github.com/abustany/pdf2svgslides";
            license = with licenses; [ gpl2 ];
          };
        };
      }
    );
}

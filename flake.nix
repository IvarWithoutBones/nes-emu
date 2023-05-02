{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
        rust-overlay.follows = "rust-overlay";
      };
    };
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , rust-overlay
    , crane
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      hostPlatform = pkgs.stdenvNoCC.hostPlatform;
      lib = pkgs.lib;

      rustToolchain = pkgs.rust-bin.stable.latest.default;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      nes-emu = pkgs.callPackage
        ({ lib
           # Linux
         , cmake
         , pkg-config
         , libxkbcommon
         , libGL
         , fontconfig
         , wayland
         , libXcursor
         , libXrandr
         , libXi
         , libX11
           # GTK, for the file picker
         , wrapGAppsHook
         , glib
         , atk
         , gtk3
         , cairo
         , pango
         , gdk-pixbuf
           # Darwin
         , AppKit
         , OpenGL
         }:
          craneLib.buildPackage {
            pname = "nes-emu";
            version =
              let
                year = lib.substring 0 4 self.lastModifiedDate;
                month = lib.substring 4 2 self.lastModifiedDate;
                day = lib.substring 6 2 self.lastModifiedDate;
              in
              "0.pre+date=${year}-${month}-${day}";

            src = craneLib.cleanCargoSource ./.;

            nativeBuildInputs = lib.optionals hostPlatform.isLinux [
              cmake
              pkg-config
              wrapGAppsHook
              glib
            ];

            buildInputs = lib.optionals hostPlatform.isLinux [
              libX11
              libXrandr
              libXcursor
              libxkbcommon
              libXi
              libGL
              fontconfig
              wayland
              cairo
              pango
              gdk-pixbuf
              atk
              gtk3
            ] ++ lib.optionals hostPlatform.isDarwin [
              AppKit
              OpenGL
            ];

            meta = with lib; {
              license = licenses.asl20;
              platforms = platforms.unix;
            };
          })
        { inherit (pkgs.darwin.apple_sdk.frameworks) AppKit OpenGL; };
    in
    {
      packages.default = nes-emu;

      checks = {
        inherit nes-emu;
      };

      devShells.default = pkgs.mkShell {
        inputsFrom = [ nes-emu ];

        packages = [
          rustToolchain
          rustToolchain.availableComponents.rust-analyzer
        ];

        LD_LIBRARY_PATH = lib.optional hostPlatform.isLinux
          (lib.makeLibraryPath nes-emu.buildInputs);

        # Avoid not being able to find gsettings schemas when opening the file picker
        shellHook = lib.optionalString hostPlatform.isLinux ''
          export XDG_DATA_DIRS="$XDG_DATA_DIRS:$GSETTINGS_SCHEMAS_PATH"
        '';
      };
    });
}

{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , rust-overlay
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };
      lib = pkgs.lib;

      rust = pkgs.rust-bin.stable.latest.default;

      nes-emu = pkgs.callPackage
        ({ lib
         , stdenvNoCC
         , rustPlatform
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
           # Darwin
         , AppKit
         , OpenGL
           # GTK
         , wrapGAppsHook
         , glib
         , atk
         , gtk3
         , cairo
         , pango
         , gdk-pixbuf
         }:
          rustPlatform.buildRustPackage {
            pname = "nes-emu";
            version =
              let
                year = lib.substring 0 4 self.lastModifiedDate;
                month = lib.substring 4 2 self.lastModifiedDate;
                day = lib.substring 6 2 self.lastModifiedDate;
              in
              "0.pre+date=${year}-${month}-${day}";

            src = lib.cleanSourceWith {
              src = lib.cleanSource ./.;
              filter = name: type:
                !(baseNameOf name == "target" && type == "directory") &&
                !(baseNameOf name == "flake.nix" && type == "file") &&
                !(baseNameOf name == "flake.lock" && type == "file");
            };

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [
              rust
            ] ++ lib.optionals stdenvNoCC.hostPlatform.isLinux [
              cmake
              pkg-config
              wrapGAppsHook
              glib
            ];

            buildInputs = lib.optionals stdenvNoCC.hostPlatform.isLinux [
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
            ] ++ lib.optionals stdenvNoCC.hostPlatform.isDarwin [
              AppKit
              OpenGL
            ];

            # TODO: remove
            doCheck = false;

            meta = with lib; {
              license = licenses.asl20;
              platforms = platforms.unix;
            };
          })
        { inherit (pkgs.darwin.apple_sdk.frameworks) AppKit OpenGL; };
    in
    {
      packages = {
        inherit nes-emu;
        default = nes-emu;
      };

      devShell = pkgs.mkShell {
        inputsFrom = [ nes-emu ];

        packages = [
          rust
          nes-emu
        ];

        LD_LIBRARY_PATH = lib.optional pkgs.stdenvNoCC.hostPlatform.isLinux
          (lib.makeLibraryPath nes-emu.buildInputs);

        # Avoid not being able to find gsettings schemas when opening the file picker
        shellHook = ''
          export XDG_DATA_DIRS="$XDG_DATA_DIRS:$GSETTINGS_SCHEMAS_PATH"
        '';
      };
    });
}

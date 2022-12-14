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
      rust = pkgs.rust-bin.stable.latest.default;

      nes-emu = pkgs.callPackage
        ({ lib
         , stdenvNoCC
         , rustPlatform
           # Linux specific
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
           # Darwin specific
         , AppKit
         , OpenGL
         }:
          rustPlatform.buildRustPackage rec {
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
                !(baseNameOf name == "target" && type == "directory");
            };

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [
              rust
            ] ++ lib.optionals stdenvNoCC.hostPlatform.isLinux [
              cmake
              pkg-config
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
            ] ++ lib.optionals stdenvNoCC.hostPlatform.isDarwin [
              AppKit
              OpenGL
            ];

            LD_LIBRARY_PATH = lib.optional stdenvNoCC.hostPlatform.isLinux (lib.makeLibraryPath buildInputs);

            meta = with lib; {
              license = licenses.asl20;
              platforms = platforms.unix;
            };
          })
        { inherit (pkgs.darwin.apple_sdk.frameworks) AppKit OpenGL; };
    in
    {
      packages = {
        inherit nes-emu rust;
        default = nes-emu;
      };
    });
}

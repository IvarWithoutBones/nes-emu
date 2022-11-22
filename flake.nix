rec {
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
         , rustPlatform
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
                !(baseNameOf name == "target" && type == "directory");
            };

            cargoLock.lockFile = ./Cargo.lock;

            meta = with lib; {
              license = licenses.asl20;
              platforms = platforms.unix;
            };
          })
        { };
    in
    {
      packages = {
        inherit nes-emu rust;
        default = nes-emu;
      };
    });
}

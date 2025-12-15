{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    # NUR Rust toolchains and rust analyzer nightly for nix.
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      flake-utils,
      nixpkgs,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };
        lib = pkgs.lib;
        pkgBuildInputs = with pkgs; [ libgcc ];
      in
      {
        devShells = {
          default = pkgs.mkShell {
            buildInputs =
              with pkgs;
              [ ]
              ++ pkgBuildInputs
              ++ (with pkgs.fenix; [
                (combine [
                  stable.cargo
                  stable.rustc
                  targets.x86_64-unknown-linux-musl.stable.rust-std
                ])
                rust-analyzer
              ]);
            shellHook = ''
              export TARGET_CC="${pkgs.musl.dev}/bin/musl-gcc"
              export TARGET_AR="ar rcus"
              export TARGET_STRIP="strip"
              export TARGET_LD="$TARGET_CC"
            '';

            LD_LIBRARY_PATH = "${lib.makeLibraryPath pkgBuildInputs}";
          };
        };
        pkgs = pkgs;
      }
    );
}

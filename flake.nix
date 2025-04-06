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

  outputs = { flake-utils, nixpkgs, fenix, ... }@inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ fenix.overlays.default ];
          };
          lib = pkgs.lib;

          pkgBuildInputs = with pkgs; [ ];
        in
        {
          devShells = {
            default = pkgs.mkShell rec {
              buildInputs = with pkgs; [ ] ++ pkgBuildInputs ++ (
                with pkgs.fenix; [
                  stable.toolchain
                  rust-analyzer
                ]
              );
              LD_LIBRARY_PATH = "${lib.makeLibraryPath pkgBuildInputs}";
            };
          };
        });
}


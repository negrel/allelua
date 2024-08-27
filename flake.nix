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

  outputs = { flake-utils, nixpkgs, fenix, self, ... }@inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ fenix.overlays.default ];
          };
          lib = pkgs.lib;
        in
        {
          packages = {
            luajit = pkgs.luajit.override { enable52Compat = true; };
          };
          devShells = {
            default = pkgs.mkShell rec {
              buildInputs = (with pkgs; [
                clang-tools
                clang
                check
                valgrind
                pkg-config
                cargo-expand
                tokio-console
              ])
              ++ (with self.packages.${system}; [ luajit ])
              ++ (with pkgs.fenix; [
                stable.toolchain
                rust-analyzer
              ]);
              LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
            };
          };
        });
}


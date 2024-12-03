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

  outputs = { flake-utils, nixpkgs, fenix, self, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };
        lib = pkgs.lib;
      in {
        packages = {
          default = pkgs.rustPlatform.buildRustPackage rec {
            pname = "allelua";
            version = "0.1.0";
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "selene-lib-0.27.1" =
                  "sha256-BcoQqim4yeEnZAgTwsFMj2AtH93tO218Z/2arhFAi9I=";
              };
            };
            nativeBuildInputs = with pkgs; [ pkg-config ];
            buildInputs = [ self.packages.${system}.luajit ];
            LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
          };
          luajit = pkgs.luajit.overrideAttrs (oldAttrs: {
            env = (oldAttrs.env or { }) // {
              NIX_CFLAGS_COMPILE = toString [
                (oldAttrs.env.NIX_CFLAGS_COMPILE or "")
                "-DLUAJIT_ENABLE_LUA52COMPAT"
              ];
              prePatch = (oldAttrs.prePatch or "") + ''
                sed -i -E 's/#define LUAI_MAXCSTACK\s+8000/#define LUAI_MAXCSTACK 0xFFFFFF00/' src/luaconf.h
                sed -i -E 's/#define LUAI_MAXSTACK\s+65500/#define LUAI_MAXSTACK 0xFFFFFF00/' src/luaconf.h
              '';
            };
          });
        };
        devShells = {
          default = pkgs.mkShell rec {
            buildInputs =
              (with pkgs; [ pkg-config cargo-expand tokio-console bats ])
              ++ (with self.packages.${system}; [ luajit ])
              ++ (with pkgs.fenix; [ stable.toolchain rust-analyzer ]);
            LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
          };
        };
      });
}


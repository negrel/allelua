{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    eventloop.url = "github:negrel/eventloop.h";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      eventloop,
      ...
    }:
    let
      outputsWithoutSystem = { };
      outputsWithSystem = flake-utils.lib.eachDefaultSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
          lib = pkgs.lib;
        in
        {
          devShells = {
            default = pkgs.mkShell rec {
              buildInputs =
                with pkgs;
                [
                  clang-tools
                  valgrind
                  pkg-config
                ]
                ++ [ self.packages.${system}.luajit ];

              EVENTLOOP_INCLUDE = "${eventloop.packages.${system}.default}/include";
              LUAJIT_INCLUDE = "${self.packages.${system}.luajit}/include";
              LUAJIT_LIB = "${self.packages.${system}.luajit}/lib";
              LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
            };
          };
          packages = {
            luajit = pkgs.luajit.overrideAttrs (oldAttrs: {
              env = (oldAttrs.env or { }) // {
                NIX_CFLAGS_COMPILE = toString [
                  (oldAttrs.env.NIX_CFLAGS_COMPILE or "")
                  "-DLUAJIT_ENABLE_LUA52COMPAT"
                ];
              };
            });
          };
        }
      );
    in
    outputsWithSystem // outputsWithoutSystem;
}

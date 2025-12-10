{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    eventloop.url = "github:negrel/eventloop.h";
  };

  outputs =
    {
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
        in
        {
          devShells = {
            default = pkgs.mkShell {
              buildInputs = with pkgs; [
                clang-tools
                valgrind
              ];

              EVENTLOOP_INCLUDE = "${eventloop.packages.${system}.default}/include";
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

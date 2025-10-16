let
  sources = import ./npins;
  pkgs = import sources.nixpkgs { };
  wire = import sources.wire;
in
pkgs.mkShell {
  packages = [
    wire.packages.${builtins.currentSystem}.wire
    pkgs.npins
  ];
}

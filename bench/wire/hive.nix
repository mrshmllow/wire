let
  flake = import ../../default.nix;
  wire = import flake;
in
wire.makeHive (import ../default.nix { inherit flake; })

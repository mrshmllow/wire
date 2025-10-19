let
  flake = import ../../default.nix;
in
import ../default.nix { inherit flake; }

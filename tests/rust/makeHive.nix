{
  nixosConfigurations ? { },
  ...
}@hive:
import ../../runtime/evaluate.nix {
  inherit
    hive
    nixosConfigurations
    ;
}

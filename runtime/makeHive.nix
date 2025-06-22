{
  nixosConfigurations ? { },
  ...
}@hive:
import ./evaluate.nix {
  inherit
    nixosConfigurations
    ;

  hive = builtins.removeAttrs hive [ "nixosConfigurations" ];
}

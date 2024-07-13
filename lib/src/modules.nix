{
  deploymentOptions = {
    name,
    lib,
    ...
  }: let
    inherit (lib) types;
  in {
    options.deployment = {
      targetHost = lib.mkOption {
        type = types.nullOr types.str;
        default = name;
      };

      targetHosts = lib.mkOption {
        type = types.listOf types.str;
        default = lib.singleton name;
        apply = list: lib.unique ([name] ++ list);
      };

      tags = lib.mkOption {
        type = types.listOf types.str;
        default = [];
      };
    };
  };
}

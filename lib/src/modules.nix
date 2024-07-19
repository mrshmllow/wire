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

      target = lib.mkOption {
        type = types.submodule {
          options = {
            host = lib.mkOption {
              type = types.str;
            };
            user = lib.mkOption {
              type = types.str;
            };
            port = lib.mkOption {
              type = types.int;
              default = 22;
            };
          };
        };
      };

      buildOnTarget = lib.mkOption {
        type = types.bool;
        default = true;
      };

      tags = lib.mkOption {
        type = types.listOf types.str;
        default = [];
      };
    };
  };
}

{hivePath}: let
  modules = import ./modules.nix;
  hive = import hivePath;

  mergedHive =
    {
      meta = {};

      defaults = {};
    }
    // hive;

  nodeNames = with builtins; filter (name: !elem name ["meta" "defaults"]) (attrNames hive);

  nixpkgs = import mergedHive.meta.nixpkgs {};

  evaluateNode = name: let
    evalConfig = import (nixpkgs.path + "/nixos/lib/eval-config.nix");
  in
    evalConfig {
      modules = [
        modules.deploymentOptions

        mergedHive.defaults
        mergedHive.${name}
      ];

      specialArgs =
        {
          inherit name;
          nodes = nodeNames;
        }
        // mergedHive.meta.specialArgs or {};
    };

  getTopLevel = node: (evaluateNode node).config.system.build.toplevel.drvPath;
in rec {
  inherit evaluateNode getTopLevel;

  nodes =
    builtins.listToAttrs
    (map (name: {
        inherit name;
        value = evaluateNode name;
      })
      nodeNames);

  inspect = {
    nodes = builtins.mapAttrs (_: v: v.config.deployment) nodes;
    path = hivePath;
  };
}

{
  hive,
  path,
}: let
  modules = import ./modules.nix;

  mergedHive =
    {
      meta = {};

      defaults = {};
    }
    // hive;

  nodeNames = with builtins; filter (name: !elem name ["meta" "defaults"]) (attrNames hive);

  # support '<nixpkgs>' and 'import <nixpkgs> {}'
  nixpkgs =
    if builtins.isPath mergedHive.meta.nixpkgs
    then import mergedHive.meta.nixpkgs {}
    else mergedHive.meta.nixpkgs;

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
    inherit path;
    nodes = builtins.mapAttrs (_: v: v.config.deployment) nodes;
  };
}

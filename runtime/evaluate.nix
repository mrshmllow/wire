# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright 2024-2025 wire Contributors

{
  hive,
  nixosConfigurations ? { },
}:
let
  module = import ./module;

  mergedHive = {
    meta = { };

    defaults = { };
  }
  // hive
  # Map nixosConfigurations into nodes
  // (builtins.mapAttrs (name: value: {
    imports =
      value._module.args.modules
      # Include any custom stuff within `colmena`
      ++ [ hive.${name} or { } ];
  }) nixosConfigurations);

  nodeNames = builtins.filter (
    name:
    !builtins.elem name [
      "meta"
      "defaults"
      "-"
    ]
  ) (builtins.filter (name: builtins.hasAttr name hive) (builtins.attrNames mergedHive));

  resolvedNixpkgs =
    if mergedHive.meta ? "nixpkgs" then
      # support `<nixpkgs>`
      if builtins.isPath mergedHive.meta.nixpkgs then
        import mergedHive.meta.nixpkgs { }
      # support npins sources passed directly
      else if mergedHive.meta.nixpkgs ? "outPath" then
        import mergedHive.meta.nixpkgs { }
      # support `import <nixpkgs> { }`
      else if builtins.isAttrs mergedHive.meta.nixpkgs then
        mergedHive.meta.nixpkgs
      else
        builtins.abort "meta.nixpkgs was not a path, { outPath, .. }, or attrset. Was type: ${builtins.typeOf mergedHive.meta.nixpkgs}"
    else
      builtins.abort "makeHive called without meta.nixpkgs specified.";

  evaluateNode =
    name:
    let
      evalConfig = import (resolvedNixpkgs.path + "/nixos/lib/eval-config.nix");
    in
    evalConfig {
      modules = [
        module

        mergedHive.defaults
        mergedHive.${name}
      ];
      system = null;
      specialArgs = {
        inherit name nodes;
      }
      // mergedHive.meta.specialArgs or { };
    };
  nodes = builtins.listToAttrs (
    map (name: {
      inherit name;
      value = evaluateNode name;
    }) nodeNames
  );

  getTopLevel = node: (evaluateNode node).config.system.build.toplevel.drvPath;
in
rec {
  inherit nodes;

  topLevels = builtins.mapAttrs (name: _: getTopLevel name) nodes;
  inspect = {
    _schema = 0;

    nodes = builtins.mapAttrs (_: v: v.config.deployment) nodes;
  };
}

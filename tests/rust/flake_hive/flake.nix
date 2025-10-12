# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright 2024-2025 wire Contributors

{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, self, ... }@inputs:
    let
      makeHive = import ./makeHive.nix;
    in
    {
      wire = makeHive {
        inherit (self) nixosConfigurations;
        meta.nixpkgs = import nixpkgs { localSystem = "x86_64-linux"; };

        node-a = { };
        node-b = {
          nixpkgs.hostPlatform = "x86_64-linux";
        };
      };

      nixosConfigurations = {
        # non-hive nixosConfiguration
        # will fail to evaluate if this node is ever included
        # in the hive because this system does not specify
        # hostPlatform
        system-c = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs = { inherit inputs; };
          modules = [ ];
        };

        # hive nixosConfiguration
        # its verified that these are merged because hostPlatform
        # must be specified, and its not specified in the hive
        node-a = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs = { inherit inputs; };
          modules = [
            {
              nixpkgs.hostPlatform = "x86_64-linux";
            }
          ];
        };
      };
    };
}

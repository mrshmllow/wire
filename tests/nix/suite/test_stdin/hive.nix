# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright 2024-2025 wire Contributors

let
  inherit (import ../utils.nix { testName = "test_keys-@IDENT@"; }) makeHive mkHiveNode;
in
makeHive {
  meta.nixpkgs = import <nixpkgs> { localSystem = "x86_64-linux"; };
  deployer = mkHiveNode { hostname = "deployer"; } {
    deployment.tags = [ "tag" ];
    environment.etc."a".text = "b";
  };
}

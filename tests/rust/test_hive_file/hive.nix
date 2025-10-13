# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright 2024-2025 wire Contributors

let
  inherit (import ../../..) makeHive;
in
makeHive {
  meta = {
    nixpkgs = <nixpkgs>;
  };

  node-a = {
    deployment = {
      target = {
        host = "192.168.122.96";
        user = "root";
      };
    };

    nixpkgs.hostPlatform = "x86_64-linux";
  };
}

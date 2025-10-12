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
    deployment._keys = [
      {
        name = "different-than-a";
        source = "hi";
      }
    ];

    nixpkgs.hostPlatform = "x86_64-linux";
  };
}

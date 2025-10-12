# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright 2024-2025 wire Contributors

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

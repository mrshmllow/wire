# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright 2024-2025 wire Contributors

{
  lib,
  snakeOil,
  wire-small-dev,
  config,
  pkgs,
  ...
}:
let
  inherit (lib)
    mkEnableOption
    mkMerge
    mkIf
    mkOption
    ;
  inherit (lib.types) lines;
  cfg = config._wire;
in
{
  options._wire = {
    deployer = mkEnableOption "deployment-specific settings";
    receiver = mkEnableOption "receiver-specific settings";
    testScript = mkOption {
      type = lines;
      default = "";
      description = "node-specific test script";
    };
  };

  config = mkMerge [
    (mkIf cfg.deployer {
      systemd.tmpfiles.rules = [
        "C+ /root/.ssh/id_ed25519 600 - - - ${snakeOil.snakeOilEd25519PrivateKey}"
      ];
      environment.systemPackages = [
        wire-small-dev
        pkgs.ripgrep
      ];
      # It's important to note that you should never ever use this configuration
      # for production. You are risking a MITM attack with this!
      programs.ssh.extraConfig = ''
        Host *
          StrictHostKeyChecking no
          UserKnownHostsFile /dev/null
      '';

      # owner user used to test keys on the deployer.
      # here instead of in the test case hive because we lose the wire binary when
      # applying to deployer.
      users.groups."owner" = { };
      users.users."owner" = {
        group = "owner";
        isNormalUser = true;
      };
    })
    (mkIf cfg.receiver {
      services.openssh.enable = true;
      users.users.root.openssh.authorizedKeys.keys = [ snakeOil.snakeOilEd25519PublicKey ];
      _wire.testScript = ''
        ${config.networking.hostName}.wait_for_unit("sshd.service")
      '';
    })
  ];
}

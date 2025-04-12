{
  lib,
  snakeOil,
  wire,
  config,
  ...
}:
let
  inherit (lib)
    mkEnableOption
    mkMerge
    mkIf
    ;
  cfg = config._wire;
in
{
  options._wire = {
    deployer = mkEnableOption "deployment-specific settings";
    receiver = mkEnableOption "receiver-specific settings";
  };

  config = mkMerge [
    (mkIf cfg.deployer {
      systemd.tmpfiles.rules = [
        "C+ /root/.ssh/id_ed25519 600 - - - ${snakeOil.snakeOilEd25519PrivateKey}"
      ];
      environment.systemPackages = [ wire ];
      # It's important to note that you should never ever use this configuration
      # for production. You are risking a MITM attack with this!
      programs.ssh.extraConfig = ''
        Host *
          StrictHostKeyChecking no
          UserKnownHostsFile /dev/null
      '';

    })
    (mkIf cfg.receiver {
      services.openssh.enable = true;
      users.users.root.openssh.authorizedKeys.keys = [ snakeOil.snakeOilEd25519PublicKey ];
    })
  ];
}

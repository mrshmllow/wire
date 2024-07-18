{
  meta = {
    nixpkgs = <nixpkgs>;
  };

  defaults = {pkgs, ...}: {
    environment.systemPackages = with pkgs; [
      vim
    ];
  };

  node-a = {
    deployment = {
      targetHost = "node-a";

      tags = ["test" "arm"];
    };

    system.stateVersion = "24.11";

    boot.loader.grub.enable = true;
    boot.loader.grub.device = "/dev/vda";

    fileSystems."/" = {
      device = "/dev/disk/by-uuid/22222";
      fsType = "ext4";
    };
  };

  node-b = {
    deployment = {
      targetHost = "node-b";

      tags = ["test" "x86"];
    };

    system.stateVersion = "24.11";

    boot.loader.grub.enable = true;
    boot.loader.grub.device = "/dev/vdc";

    # fileSystems."/" = {
    #   device = "/dev/disk/by-uuid/11111";
    #   fsType = "ext4";
    # };
  };
}

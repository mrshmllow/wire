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
    };

    system.stateVersion = "24.11";

    boot.loader.grub.enable = true;
    boot.loader.grub.device = "/dev/vda";

    fileSystems."/" = {
      device = "/dev/disk/by-uuid/00000";
      fsType = "ext4";
    };
  };
}

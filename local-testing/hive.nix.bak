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
      target = {
        host = "192.168.122.96";
        user = "root";
      };

      tags = ["test" "arm"];
    };

    imports = [./node-a.nix];
  };

  node-b = {
    deployment = {
      target = {
        host = "node-b";
        user = "nixos";
      };

      tags = ["test" "x86"];
    };

    system.stateVersion = "24.11";

    boot.loader.grub.enable = true;
    boot.loader.grub.device = "/dev/vdc";

    fileSystems."/" = {
      device = "/dev/disk/by-uuid/11111";
      fsType = "ext4";
    };
  };
}

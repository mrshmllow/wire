{pkgs, ...}: {
  boot.kernelPackages = pkgs.linuxPackages_latest;
  documentation.enable = false;

  services.openssh.enable = true;

  virtualisation = {
    forwardPorts = [
      {
        from = "host";
        host.port = 2222;
        guest.port = 22;
      }
    ];
  };

  # virtualisation.useNixStoreImage = true;
  virtualisation.useBootLoader = true;

  boot.loader.grub.enable = false;

  system.stateVersion = "24.05";
}

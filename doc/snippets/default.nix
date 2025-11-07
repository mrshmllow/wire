{
  getting-started-nixos = import ./getting-started/configuration.nix;
  getting-started-nixos-flake = import ./getting-started/nixos.flake.nix;
  getting-started-cache = import ./getting-started/cache.nix;
}

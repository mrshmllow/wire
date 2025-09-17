{
  nix.settings = {
    substituters = [
      "https://cache.nixos.org"
      "https://cache.althaea.zone"
      # ...
    ];
    trusted-public-keys = [
      "cache.althaea.zone:BelRpa863X9q3Y+AOnl5SM7QFzre3qb+5I7g2s/mqHI="
      # ...
    ];
  };
}

{
  nix.settings = {
    substituters = [
      "https://wires.cachix.org"
      # ...
    ];
    trusted-public-keys = [
      "wires.cachix.org-1:7XQoG91Bh+Aj01mAJi77Ui5AYyM1uEyV0h1wOomqjpk="
      # ...
    ];
  };
}
let
  makeHive = import ../makeHive.nix;
in
makeHive {
  meta = {
    nixpkgs = <nixpkgs>;
  };

  NAME = {
    nixpkgs.hostPlatform = "x86_64-linux";
  };
}

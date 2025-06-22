let
  makeHive = import ../makeHive.nix;
in
makeHive {
  meta = {
    nixpkgs = <nixpkgs>;
  };

  node-a = {
    deployment._keys = [
      {
        name = "different-than-a";
        source = "hi";
      }
    ];
  };
}

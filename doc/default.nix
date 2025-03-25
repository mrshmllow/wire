{
  perSystem =
    { pkgs, self', ... }:
    {
      packages.doc = pkgs.callPackage ./package.nix { inherit (self'.packages) wire; };
    };
}

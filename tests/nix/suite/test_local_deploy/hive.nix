let
  inherit (import ../utils.nix { testName = "test_keys-@IDENT@"; }) makeHive mkHiveNode;
in
makeHive {
  meta.nixpkgs = import <nixpkgs> { localSystem = "x86_64-linux"; };
  deployer = mkHiveNode { hostname = "deployer"; } {
    environment.etc."a".text = "b";
  };
}

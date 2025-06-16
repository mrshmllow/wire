let
  mkHiveNode = import ../utils.nix { testName = "test_local_deploy-@IDENT@"; };
in
{
  meta.nixpkgs = import <nixpkgs> { system = "x86_64-linux"; };
  deployer = mkHiveNode { hostname = "deployer"; } {
    environment.etc."a".text = "b";
  };
}

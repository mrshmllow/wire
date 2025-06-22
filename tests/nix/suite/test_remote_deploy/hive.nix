let
  inherit (import ../utils.nix { testName = "test_keys-@IDENT@"; }) makeHive mkHiveNode;
in
makeHive {
  meta.nixpkgs = import <nixpkgs> { system = "x86_64-linux"; };
  receiver = mkHiveNode { hostname = "receiver"; } {
    environment.etc."a".text = "b";

    # test node pinging
    deployment.target.hosts = [
      "unreachable-1"
      "unreachable-2"
      "unreachable-3"
      "unreachable-4"
      "receiver"
    ];
  };

  receiver-unreachable = mkHiveNode { hostname = "receiver"; } {
    # test node pinging
    deployment.target.hosts = [
      "completely-unreachable"
    ];
  };
}

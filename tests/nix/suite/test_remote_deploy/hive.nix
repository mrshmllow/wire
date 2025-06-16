let
  mkHiveNode = import ../utils.nix { testName = "test_remote_deploy-@IDENT@"; };
in
{
  meta.nixpkgs = import <nixpkgs> { system = "x86_64-linux"; };
  receiver = mkHiveNode { hostname = "receiver"; } {
    environment.etc."a".text = "b";

    users.groups."owner" = { };
    users.users."owner" = {
      group = "owner";
      isNormalUser = true;
    };

    deployment.keys = {
      source_string = {
        source = ''
          hello_world_source
        '';
      };
      file = {
        source = ./file.txt;
        destDir = "/etc/keys/";
        permissions = "644";
      };
      command = {
        source = [
          "echo"
          "hello_world_command"
        ];
        permissions = "644";
        user = "owner";
        group = "owner";
        destDir = "/home/owner/some/deep/path";
      };
    };
  };
}

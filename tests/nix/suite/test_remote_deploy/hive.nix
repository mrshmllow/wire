let
  mkHiveNode = import ../utils.nix { testName = "test_remote_deploy"; };
in
{
  meta.nixpkgs = import <nixpkgs> { system = "x86_64-linux"; };
  receiver = mkHiveNode { hostname = "receiver"; } {
    environment.etc."a".text = "b";

    deployment.keys = {
      source_string = {
        source = ''
          hello_world_source
        '';
      };
      file = {
        source = ./file.txt;
      };
      command = {
        source = [
          "echo"
          "hello_world_command"
        ];
      };
    };
  };
}

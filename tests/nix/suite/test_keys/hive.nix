let
  inherit (import ../utils.nix { testName = "test_keys-@IDENT@"; }) makeHive mkHiveNode;
in
makeHive {
  meta.nixpkgs = import <nixpkgs> { localSystem = "x86_64-linux"; };
  defaults = {
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
        # Test defaulting to root when user or group does not exist
        user = "USERDOESNOTEXIST";
        group = "USERDOESNOTEXIST";
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
      environment = {
        source = [
          "bash"
          "-c"
          "echo $ENV_STRING"
        ];
        environment.ENV_STRING = "string_from_environment";
      };
    };

    users.groups."owner" = { };
    users.users."owner" = {
      group = "owner";
      isNormalUser = true;
    };
  };

  receiver = mkHiveNode { hostname = "receiver"; } (
    { pkgs, ... }:
    {
      environment.etc."a".text = "b";
      environment.systemPackages = [ pkgs.ripgrep ];
    }
  );

  deployer = mkHiveNode { hostname = "deployer"; } {
    environment.etc."a".text = "b";
  };
}

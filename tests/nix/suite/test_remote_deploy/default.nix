{ config, ... }:
{
  wire.testing.test_remote_deploy = {
    nodes.deployer = {
      _wire.deployer = true;
    };
    nodes.receiver = {
      _wire.receiver = true;
    };
    testScript = ''
      deployer.succeed("wire apply --on receiver --no-progress --path ${config.wire.testing.test_remote_deploy.testDir}/hive.nix --no-keys -vvv >&2")

      receiver.wait_for_unit("sshd.service")

      receiver.succeed("test -f /etc/a")

      # --no-keys should never push a key
      receiver.fail("test -f /run/keys/source_string")

      # push keys
      deployer.succeed("wire apply keys --on receiver --no-progress --path ${config.wire.testing.test_remote_deploy.testDir}/hive.nix -vvv >&2")

      receiver.succeed("test -f /run/keys/source_string")
      source_string = receiver.succeed("cat /run/keys/source_string")
      assert "hello_world_source" in source_string, "source secret correct"

      receiver.succeed("test -f /run/keys/file")
      file_string = receiver.succeed("cat /run/keys/file")
      assert "hello_world_file" in file_string, "file secret correct"

      receiver.succeed("test -f /run/keys/command")
      command_string = receiver.succeed("cat /run/keys/command")
      assert "hello_world_command" in command_string, "command secret correct"
    '';
  };
}

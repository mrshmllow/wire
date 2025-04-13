{ config, ... }:
{
  wire.testing.test_basic_deploy = {
    nodes.deployer = {
      _wire.deployer = true;
    };
    nodes.receiver = {
      _wire.receiver = true;
    };
    testScript = ''
      deployer.succeed("wire apply --on receiver --no-progress --path ${config.wire.testing.test_basic_deploy.testDir}/hive.nix --no-keys -vvv >&2")
      receiver.succeed("test -f /etc/a")
    '';
  };
}

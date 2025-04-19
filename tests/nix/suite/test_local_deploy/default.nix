{ config, ... }:
{
  wire.testing.test_local_deploy = {
    nodes.deployer = {
      _wire.deployer = true;
      _wire.receiver = true;
    };
    testScript = ''
      deployer.succeed("wire apply --on deployer --no-progress --path ${config.wire.testing.test_local_deploy.testDir}/hive.nix --no-keys -vvv >&2")
      deployer.succeed("test -f /etc/a")
    '';
  };
}

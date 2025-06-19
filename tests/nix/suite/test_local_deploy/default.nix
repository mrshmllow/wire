{
  wire.testing.test_local_deploy = {
    nodes.deployer = {
      _wire.deployer = true;
      _wire.receiver = true;
    };
    testScript = ''
      deployer.succeed(f"wire apply --on deployer --no-progress --path {TEST_DIR}/hive.nix --no-keys -vvv >&2")
      deployer.succeed("test -f /etc/a")
    '';
  };
}

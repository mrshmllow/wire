{
  wire.testing.test_remote_deploy = {
    nodes.deployer = {
      _wire.deployer = true;
    };
    nodes.receiver = {
      _wire.receiver = true;
    };
    testScript = ''
      deployer_so = collect_store_objects(deployer)
      receiver_so = collect_store_objects(receiver)

      deployer.succeed(f"wire apply --on receiver --no-progress --path {TEST_DIR}/hive.nix --no-keys -vvv >&2")

      receiver.wait_for_unit("sshd.service")

      receiver.succeed("test -f /etc/a")
    '';
  };
}

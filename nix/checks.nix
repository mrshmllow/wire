{
  perSystem =
    {
      craneLib,
      commonArgs,
      ...
    }:
    {
      checks.wire-nextest = craneLib.cargoNextest (
        {
          partitions = 2;
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          cargoNextestPartitionsExtraArgs = builtins.concatStringsSep " " [
            "--no-tests pass"
          ];

        }
        // commonArgs
      );
    };
}

{
  perSystem =
    {
      craneLib,
      commonArgs,
      self',
      ...
    }:
    {
      checks = {
        inherit (self'.packages) wire wire-small docs;

        wire-nextest = craneLib.cargoNextest (
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
    };
}

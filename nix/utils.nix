{
  perSystem =
    {
      pkgs,
      lib,
      craneLib,
      ...
    }:
    {
      _module.args = {
        buildRustProgram =
          extra:
          let
            args = builtins.removeAttrs extra [
              "name"
            ];
            # FIXME: may be a better way of going about this
            src = ../.;

            commonArgs = {
              inherit src;
              strictDeps = true;
              WIRE_RUNTIME = ../runtime;
              WIRE_TEST_DIR = ../tests/rust;
              PROTOC = lib.getExe pkgs.protobuf;
            };
          in
          craneLib.buildPackage (
            {
              cargoArtifacts = craneLib.buildDepsOnly commonArgs;
            }
            // args
            // commonArgs
          );
      };
    };
}

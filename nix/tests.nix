{
  perSystem =
    {
      craneLib,
      pkgs,
      commonArgs,
      ...
    }:
    let
      tests = craneLib.buildPackage (
        {
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          doCheck = false;

          doNotPostBuildInstallCargoBinaries = true;

          buildPhase = ''
            cargo test --no-run
          '';

          installPhaseCommand = ''
            mkdir -p $out
            cp $(ls target/debug/deps/{wire,lib,key_agent}-* | grep -v "\.d") $out
          '';
        }
        // commonArgs
      );
    in
    {
      packages.cargo-test = pkgs.writeShellScriptBin "run-tests" ''
        for item in "${tests}"/*; do
            echo "running $item"
            "$item"
        done
      '';
    };
}

{
  lib,
  craneLib,
}: let
  src = craneLib.cleanCargoSource ./.;

  commonArgs = {
    inherit src;
    strictDeps = true;
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  individualCrateArgs =
    commonArgs
    // {
      inherit cargoArtifacts;
      inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
    };

  fileSetForCrate = crate:
    lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./Cargo.toml
        ./Cargo.lock
        ./wire
        ./lib
        crate
      ];
    };
in
  craneLib.buildPackage (individualCrateArgs
    // rec {
      pname = "wire";
      cargoExtraArgs = "-p ${pname}";
      src = fileSetForCrate ./wire;
      meta.mainProgram = pname;
    })

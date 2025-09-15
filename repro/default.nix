{
  perSystem =
    {
      pkgs,
      buildRustProgram,
      ...
    }:
    {
      packages = {
        repro = buildRustProgram {
          name = "repro";
          pname = "repro";
          cargoExtraArgs = "-p repro";
          doCheck = false;
          nativeBuildInputs = [ pkgs.installShellFiles ];
        };
      };
    };
}

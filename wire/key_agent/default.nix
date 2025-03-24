{
  perSystem =
    {
      buildRustProgram,
      ...
    }:
    {
      packages = {
        agent = buildRustProgram {
          name = "key_agent";
          pname = "key_agent";
          cargoExtraArgs = "-p key_agent";
        };
      };
    };
}

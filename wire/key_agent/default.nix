{
  perSystem =
    {
      buildRustProgram,
      system,
      ...
    }:
    {
      packages = {
        agent = buildRustProgram {
          name = "key_agent";
          pname = "wire-tool-key_agent-${system}";
          cargoExtraArgs = "-p key_agent";
        };
      };
    };
}

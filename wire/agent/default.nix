{
  config,
  withSystem,
  lib,
  ...
}:
{

  flake.packages =
    # macos is not supported with the agent
    lib.genAttrs (builtins.filter (system: !lib.hasSuffix "darwin" system) config.systems)

      (
        system:
        withSystem system (
          { buildRustProgram, ... }:
          {
            agent = buildRustProgram {
              name = "agent";
              pname = "agent";
              cargoExtraArgs = "-p agent";
            };
          }
        )
      );
}

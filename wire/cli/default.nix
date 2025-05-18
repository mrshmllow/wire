{ getSystem, inputs, ... }:
{
  perSystem =
    {
      pkgs,
      lib,
      self',
      buildRustProgram,
      ...
    }:
    let
      agents = lib.strings.concatMapStrings (
        system:
        "--set WIRE_KEY_AGENT_${
          lib.replaceStrings [ "-" ] [ "_" ] system
        } ${(getSystem system).packages.agent} "
      ) (import inputs.linux-systems);
    in
    {
      packages = {
        default = self'.packages.wire;
        wire-unwrapped = buildRustProgram {
          name = "wire";
          pname = "wire";
          cargoExtraArgs = "-p wire";
          doCheck = true;
          nativeBuildInputs = [ pkgs.installShellFiles ];
          postInstall = ''
            installShellCompletion --cmd wire \
                --bash <($out/bin/wire completions bash) \
                --fish <($out/bin/wire completions fish) \
                --zsh <($out/bin/wire completions zsh)
          '';
        };

        wire = pkgs.symlinkJoin {
          name = "wire";
          paths = [ self'.packages.wire-unwrapped ];
          nativeBuildInputs = [
            pkgs.makeWrapper
          ];
          postBuild = ''
            wrapProgram $out/bin/wire \
                --set WIRE_RUNTIME ${../../runtime} \
                --set WIRE_KEY_AGENT ${self'.packages.agent} \
                ${agents}
          '';
          meta.mainProgram = "wire";
        };
      };
    };
}

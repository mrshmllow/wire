{ self, ... }:
{
  perSystem =
    {
      pkgs,
      lib,
      self',
      buildRustProgram,
      ...
    }:
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
          postBuild =
            let
              agents = lib.mapAttrsToList (name: value: {
                inherit name;
                path = value.agent;
              }) (lib.filterAttrs (_: value: value ? agent) self.packages);

            in
            ''
              wrapProgram $out/bin/wire \
                  --set WIRE_RUNTIME ${../../runtime} \
                  --set WIRE_AGENT ${pkgs.linkFarm "wire-agents-farm" agents}
            '';
          meta.mainProgram = "wire";
        };
      };
    };
}

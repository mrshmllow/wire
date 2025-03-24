{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-compat.url = "github:edolstra/flake-compat";
    git-hooks.url = "github:cachix/git-hooks.nix";
    systems.url = "github:nix-systems/default";
  };
  outputs =
    {
      flake-parts,
      systems,
      git-hooks,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        git-hooks.flakeModule
        ./nix/hooks.nix # pre-commit hooks
      ];
      systems = import systems;

      perSystem =
        { pkgs, ... }:
        {
          formatter = pkgs.nixfmt-rfc-style;
        };

    };
}

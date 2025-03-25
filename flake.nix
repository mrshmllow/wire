{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-compat.url = "github:edolstra/flake-compat";
    git-hooks.url = "github:cachix/git-hooks.nix";
    systems.url = "github:nix-systems/default";
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix.url = "github:nix-community/fenix";
  };
  outputs =
    {
      flake-parts,
      systems,
      git-hooks,
      crane,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        git-hooks.flakeModule
        ./nix/hooks.nix # pre-commit hooks
        ./nix/utils.nix # utility functions
        ./wire/cli
        ./wire/key_agent
        ./doc
      ];
      systems = import systems;

      perSystem =
        {
          pkgs,
          inputs',
          config,
          ...
        }:
        {
          _module.args = {
            toolchain = inputs'.fenix.packages.complete;
            craneLib = (crane.mkLib pkgs).overrideToolchain config._module.args.toolchain.toolchain;
          };
          formatter = pkgs.nixfmt-rfc-style;
        };

    };
}

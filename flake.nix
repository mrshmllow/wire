{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    devenv.url = "github:cachix/devenv";
    fenix = {
      url = "github:nix-community/fenix";
      inputs = {nixpkgs.follows = "nixpkgs";};
    };
  };

  outputs = {
    self,
    nixpkgs,
    devenv,
    ...
  } @ inputs: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-linux" "x86_64-darwin" "i686-linux" "aarch64-linux"];
  in {
    packages = forAllSystems (system: {
      devenv-up = self.devShells.${system}.default.config.procfileScript;

      wire = nixpkgs.legacyPackages.${system}.callPackage ./default.nix {};

      default = self.packages.${system}.wire;
    });

    devShells = forAllSystems (system: {
      default = devenv.lib.mkShell {
        inherit inputs;
        pkgs = nixpkgs.legacyPackages.${system};
        modules = [
          {
            languages.rust.enable = true;
            languages.rust.channel = "nightly";

            pre-commit.hooks = {
              clippy.enable = true;
              cargo-check.enable = true;
              alejandra.enable = true;
              statix.enable = true;
              deadnix.enable = true;
            };
          }
        ];
      };
    });
  };
}

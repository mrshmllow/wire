{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    devenv.url = "github:cachix/devenv";
    fenix = {
      url = "github:nix-community/fenix";
      inputs = {nixpkgs.follows = "nixpkgs";};
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    catppuccin.url = "github:catppuccin/mdBook";
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
    crane,
    devenv,
    catppuccin,
    ...
  } @ inputs: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-linux" "x86_64-darwin" "i686-linux" "aarch64-linux"];
  in {
    packages = forAllSystems (system: {
      devenv-up = self.devShells.${system}.default.config.procfileScript;

      docs = nixpkgs.legacyPackages.${system}.callPackage ./doc {
        inherit catppuccin;
        inherit (self.packages.${system}) wire;
      };

      wire = let
        craneLib =
          (crane.mkLib nixpkgs.legacyPackages.${system}).overrideToolchain
          fenix.packages.${system}.minimal.toolchain;
      in
        nixpkgs.legacyPackages.${system}.callPackage ./default.nix {
          inherit craneLib;
        };

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

            packages = with nixpkgs.legacyPackages.${system}; [mdbook catppuccin.packages.${system}.default];

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

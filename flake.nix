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
      inputs = {
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
      };
    };
    catppuccin.url = "github:catppuccin/mdBook";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    devenv,
    fenix,
    catppuccin,
    ...
  } @ inputs: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-linux" "x86_64-darwin" "i686-linux" "aarch64-linux"];
  in {
    packages = forAllSystems (system: {
      devenv-up = self.devShells.${system}.default.config.procfileScript;

      wire = let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib =
          (crane.mkLib pkgs).overrideToolchain (_p:
            fenix.packages.${system}.minimal.toolchain);

        src = craneLib.cleanCargoSource ./.;
        commonArgs = {
          inherit src;
          strictDeps = true;
          pname = "wire";
          WIRE_RUNTIME = ./runtime;
          WIRE_TEST_DIR = ./tests;

          nativeBuildInputs = with pkgs; [
            pkgs.nix
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        package = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;
          });
      in
        pkgs.symlinkJoin {
          name = "wire";
          paths = [package];
          buildInputs = [pkgs.makeWrapper];
          postBuild = ''
            wrapProgram $out/bin/wire --set WIRE_RUNTIME ${./runtime}
          '';
          meta = {
            mainProgram = "wire";
          };
        };

      docs = nixpkgs.legacyPackages.${system}.callPackage ./doc {
        inherit catppuccin;
        inherit (self.packages.${system}) wire;
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

            env.WIRE_RUNTIME = ./runtime;

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

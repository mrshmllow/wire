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
    nixos-shell.url = "github:Mic92/nixos-shell";
    flake-compat.url = "github:edolstra/flake-compat";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    devenv,
    fenix,
    catppuccin,
    nixos-shell,
    ...
  } @ inputs: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-linux" "x86_64-darwin" "i686-linux" "aarch64-linux"];
  in {
    packages = forAllSystems (system: {
      devenv-up = self.devShells.${system}.default.config.procfileScript;

      wire = let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;
        craneLib =
          (crane.mkLib pkgs).overrideToolchain (_p:
            fenix.packages.${system}.minimal.toolchain);

        src = lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (lib.hasSuffix "\.proto" path)
            || (craneLib.filterCargoSources path type);
        };

        commonArgs = {
          inherit src;
          strictDeps = true;

          WIRE_RUNTIME = ./runtime;
          WIRE_TEST_DIR = ./tests/rust;
          PROTOC = lib.getExe pkgs.protobuf;

          nativeBuildInputs = with pkgs; [
            nix
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        agent = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;
            pname = "key_agent";
            cargoExtraArgs = "-p key_agent";
          });

        package = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;
            pname = "wire";
            cargoExtraArgs = "-p wire";
          });
      in
        pkgs.symlinkJoin {
          name = "wire";
          paths = [package];
          buildInputs = [pkgs.makeWrapper];
          postBuild = ''
            wrapProgram $out/bin/wire --set WIRE_RUNTIME ${./runtime} --set WIRE_KEY_AGENT ${agent}
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

    checks = forAllSystems (
      system: {
        nixos-tests = import ./intergration-testing/default.nix {
          inherit (self.packages.${system}) wire;
          pkgs = nixpkgs.legacyPackages.${system};
        };
      }
    );

    devShells = forAllSystems (system: {
      default = let
        pkgs = nixpkgs.legacyPackages.${system};
      in
        devenv.lib.mkShell {
          inherit inputs pkgs;
          modules = [
            {
              languages.rust.enable = true;
              languages.rust.channel = "nightly";

              env = {
                WIRE_RUNTIME = ./runtime;
                WIRE_TEST_DIR = ./tests;
                PROTOC = nixpkgs.lib.getExe pkgs.protobuf;
              };

              packages = with pkgs; [mdbook catppuccin.packages.${system}.default protobuf just pkgs.nixos-shell];

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

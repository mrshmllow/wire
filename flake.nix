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
    nixos-shell.url = "github:Mic92/nixos-shell";
    flake-compat.url = "github:edolstra/flake-compat";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    devenv,
    fenix,
    nixos-shell,
    ...
  } @ inputs: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-linux" "x86_64-darwin" "i686-linux" "aarch64-linux"];
    env = pkgs: {
      WIRE_RUNTIME = ./runtime;
      WIRE_TEST_DIR = ./tests/rust;
      PROTOC = pkgs.lib.getExe pkgs.protobuf;
    };
    hooks = {
      clippy.enable = true;
      cargo-check.enable = true;
      rustfmt.enable = true;
      alejandra.enable = true;
      statix.enable = true;
      deadnix.enable = true;
    };
    mkCrane = system: let
      pkgs = nixpkgs.legacyPackages.${system};
      inherit (pkgs) lib;
    in rec {
      craneLib =
        (crane.mkLib pkgs).overrideToolchain (_p:
          fenix.packages.${system}.minimal.toolchain);

      src = lib.cleanSourceWith {
        src = ./.;
        filter = path: type:
          (lib.hasSuffix "\.proto" path)
          || (craneLib.filterCargoSources path type);
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      commonArgs =
        {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            nix
          ];
        }
        // (env pkgs);
    };
  in {
    packages = forAllSystems (system: {
      devenv-up = self.devShells.${system}.default.config.procfileScript;

      wire = let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (mkCrane system) craneLib commonArgs cargoArtifacts;

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
            nativeBuildInputs = [pkgs.installShellFiles];
            doCheck = true;
            postInstall = ''
              $out/bin/wire apply --generate-completions bash > wire.bash
              $out/bin/wire apply --generate-completions fish > wire.fish
              $out/bin/wire apply --generate-completions zsh > wire.zsh
              installShellCompletion wire.{bash,fish,zsh}
            '';
          });
      in
        pkgs.symlinkJoin {
          name = "wire";
          paths = [package];
          buildInputs = [pkgs.makeWrapper];
          nativeBuildInputs = [pkgs.installShellFiles];
          postBuild = ''
            wrapProgram $out/bin/wire --set WIRE_RUNTIME ${./runtime} --set WIRE_KEY_AGENT ${agent}
          '';
          meta = {
            mainProgram = "wire";
          };
        };

      docs = nixpkgs.legacyPackages.${system}.callPackage ./doc {
        inherit (self.packages.${system}) wire;
      };

      default = self.packages.${system}.wire;
    });

    checks = forAllSystems (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        toolchain = fenix.packages.${system}.complete;
        inherit (mkCrane system) craneLib commonArgs cargoArtifacts;
      in {
        nixos-tests = import ./intergration-testing/default.nix {
          inherit (self.packages.${system}) wire;
          inherit pkgs;
        };

        wire-nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            cargoNextestPartitionsExtraArgs = "--no-tests=pass --features no_web_tests";
          });

        pre-commit-check =
          (devenv.inputs.git-hooks.lib.${system}.run {
            src = ./.;
            hooks =
              hooks
              // {
                clippy = {
                  enable = true;
                  packageOverrides.cargo = toolchain.cargo;
                  packageOverrides.clippy = toolchain.clippy;
                };
                cargo-check = {
                  enable = true;
                  package = toolchain.cargo;
                };
              };

            settings.rust.check.cargoDeps = pkgs.rustPlatform.importCargoLock {
              lockFile = ./Cargo.lock;
            };
          })
          .overrideAttrs (env pkgs);
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

              env = env pkgs;
              packages = with pkgs; [mdbook protobuf just pkgs.nixos-shell cargo-nextest];

              pre-commit.hooks = hooks;
            }
          ];
        };
    });
  };
}

{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
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
    flake-compat.url = "github:edolstra/flake-compat";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    git-hooks,
    fenix,
    ...
  }: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-linux" "x86_64-darwin" "i686-linux" "aarch64-linux"];
    env = pkgs: {
      WIRE_RUNTIME = ./runtime;
      WIRE_TEST_DIR = ./tests/rust;
      PROTOC = pkgs.lib.getExe pkgs.protobuf;
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

      agent = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
          pname = "key_agent";
          cargoExtraArgs = "-p key_agent";
        });

      wire = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
          pname = "wire";
          cargoExtraArgs = "-p wire";
          nativeBuildInputs = [pkgs.installShellFiles];
          doCheck = true;
          postInstall = ''
            installShellCompletion --cmd wire \
                --bash <($out/bin/wire completions bash) \
                --fish <($out/bin/wire completions fish) \
                --zsh <($out/bin/wire completions zsh)
          '';
        });
    };
    _pre-commit-check = system: let
      pkgs = nixpkgs.legacyPackages.${system};
      toolchain = fenix.packages.${system}.complete;
    in
      git-hooks.lib.${system}.run {
        src = ./.;
        hooks = {
          rustfmt.enable = true;
          alejandra.enable = true;
          statix.enable = true;
          deadnix.enable = true;
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
      };
  in {
    packages = forAllSystems (system: {
      wire = let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (mkCrane system) wire agent;
      in
        pkgs.symlinkJoin {
          name = "wire";
          paths = [wire];
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

        pre-commit-check = (_pre-commit-check system).overrideAttrs (env pkgs);
      }
    );

    devShells = forAllSystems (system: {
      default = let
        inherit (mkCrane system) craneLib wire;
        pkgs = nixpkgs.legacyPackages.${system};
        pre-commit-check = _pre-commit-check system;
      in
        craneLib.devShell ({
            inherit (pre-commit-check) shellHook;
            buildInputs = pre-commit-check.enabledPackages;

            inputsFrom = [wire];

            packages = with pkgs; [mdbook protobuf just cargo-nextest];
          }
          // (env pkgs));
    });
  };
}

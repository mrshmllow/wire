{
  perSystem =
    {
      config,
      lib,
      craneLib,
      pkgs,
      ...
    }:
    let
      cfg = config.pre-commit;
    in
    {
      # Adapted from
      # https://github.com/cachix/git-hooks.nix/blob/dcf5072734cb576d2b0c59b2ac44f5050b5eac82/flake-module.nix#L66-L78
      devShells.default = craneLib.devShell {
        packages = lib.flatten [
          cfg.settings.enabledPackages
          cfg.settings.package

          pkgs.just
          pkgs.cargo-nextest
          pkgs.pnpm
        ];

        PROTOC = lib.getExe pkgs.protobuf;
        shellHook = builtins.concatStringsSep "\n" [
          cfg.installationScript
          ''
            export WIRE_RUNTIME=$(realpath ./runtime)
            export WIRE_TEST_DIR=$(realpath ./tests/rust)
          ''
        ];
      };
    };
}

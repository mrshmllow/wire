{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    wire.url = "github:mrshmllow/wire/v0.5.0";
    systems.url = "github:nix-systems/default";
  };

  outputs = {
    nixpkgs,
    wire,
    systems,
    ...
  }: let
    forAllSystems = nixpkgs.lib.genAttrs (import systems);
  in {
    wire = wire.makeHive {
      # ...
    };

    devShells = forAllSystems (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = pkgs.mkShell {
          buildInputs = [
            wire.packages.${system}.wire
          ];
        };
      }
    );
  };
}

{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }@inputs:
    {
      colmena = {
        node-a = {
          nixpkgs.hostPlatform = "x86_64-linux";
        };
      };

      nixosConfigurations.system-b = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inherit inputs; };
        modules = [
          {
            nixpkgs.hostPlatform = "x86_64-linux";
          }
        ];
      };
    };
}

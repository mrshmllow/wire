# With Flakes

Deploying nodes from a flake is as simple as adding a `colmena` attribute to your flake outputs.

Wire will merge nixos options in `nixosConfigurations.${nodeName}` and `colmena.${nodeName}` to create the configuration it deploys. This means you can continue to use `nixos-rebuild` to deploy your configurations alongside wire.

You don't need to create a `nixosConfigurations` attribute for your nodes if you don't want to.

Every node recieves its name and a list of all other nodes through specialArgs.

```nix
# flake.nix
{
  # wire will automatically use the nixpkgs input.
  # change `colmena.meta.nixpkgs` if you want to use a different nixpkgs
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {nixpkgs, ...}: {
    nixosConfigurations.node-a = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./node-a/configuration.nix
      ];
    };

    colmena = {
      defaults = {
        # name of node default is being applied to
        node,
        # list of all nodes
        nodes,
        pkgs,
        ...
      }: {
        environment.systemPackages = with pkgs; [
          vim
        ];
      };

      node-a = {
        # name of this node
        name,
        # list of all nodes
        nodes,
        pkgs,
        ...
      }: {
        deployment.target = {
          # host defaults to the name of the node
          host = "node-a";
          # if you use a different user, it must be wheel
          user = "root";
        };

        # wire specific options are only valid here
        # for example, adding keys
        deployment.keys."key.env" = {
          destDir = "/etc/keys/";
          source = [
            "gpg"
            "--decrypt"
            ./secrets/key.env.gpg
          ];
        };

        # other module options are valid here
        system.stateVersion = "24.11";
      };

      # nodes don't have to be a function
      # and they don't have to be in `nixosConfigurations`
      node-b = {
        deployment.target = {
          # ...
        };

        imports = [./node-b/configuration.nix];
      };
    };
  };
}
```

# Without Flakes

Create a `hive.nix` file, which is equivalent to a flake's `outputs.colmena`.
You must manually specify where nixpkgs comes from when using `hive.nix`.

Every node recieves its name and a list of all other nodes through specialArgs.

```nix
# hive.nix
{
  # you must specify where nixpkgs comes from when using hive.nix
  meta.nixpkgs = <nixpkgs>;

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

    imports = [./node-a/configuration.nix];

    # wire specific options are valid here
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
  node-b = {
    deployment.target = {
      # ...
    };

    imports = [./node-b/configuration.nix];
  };
}
```

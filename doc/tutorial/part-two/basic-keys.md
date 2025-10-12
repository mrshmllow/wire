---
comment: true
title: Deployment Keys Basics
description: Deploy a basic secret with Wire tool.
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

## Creating a `secrets.nix`

Lets create a NixOS module that will contain our secret keys.

```nix:line-numbers [hive.nix]
let
  sources = import ./npins;
  wire = import sources.wire;
in
wire.makeHive {
  meta.nixpkgs = import sources.nixpkgs { };

  virtual-machine = {
    deployment.target = {
        port = 2222;
        hosts = [ "localhost" ];
    };

    imports = [
        ./vm.nix
        ./secrets.nix # [!code ++]
    ];

    environment.systemPackages = [ pkgs.vim ];

    nixpkgs.hostPlatform = "x86_64-linux";
  };
}
```

```nix:line-numbers [secrets.nix]
{
  deployment.keys = {
    "basic.txt" = {
      source = ''
        Hello World
      '';
    };
  };
}
```

```sh
[nix-shell]$ wire apply keys
 WARN lib::nix_log: Store URL: ssh://root@localhost
(root@localhost) Password:

```

```sh [Virtual Machine]
[root@wire-tutorial:~]# cat /run/keys/basic.txt
Hello World

```

## File-sourced keys

```sh
[nix-shell]$ echo hello world > very-important-secret.txt
```

```nix:line-numbers [secrets.nix]
{
  deployment.keys = {
    # ...

    "very-important-secret.txt" = { # [!code ++]
      source = ./very-important-secret.txt; # [!code ++]
    }; # [!code ++]
  };
}
```

```sh [Virtual Machine]
[root@wire-tutorial:~]# cat /run/keys/very-important-secret.txt
hello world

```

## Command-sourced keys

```nix:line-numbers [secrets.nix]
{
  deployment.keys = {
    # ...

    "command.txt" = { # [!code ++]
      source = [ # [!code ++]
        "echo" # [!code ++]
        "hello world" # [!code ++]
      ]; # [!code ++]
    }; # [!code ++]
  };
}
```

```sh [Virtual Machine]
[root@wire-tutorial:~]# cat /run/keys/command.txt
hello world

```

Hopefully you can see the potential of command-sourced keys, as these are the
basic building block of how we achieve encrypted secrets with wire.

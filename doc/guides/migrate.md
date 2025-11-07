---
comment: true
title: Migrate to wire
description: How-to migrate from other tools to wire tool.
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

## From Colmena

::: tip
You should also follow [installation](/guides/installation) to install the
binary.
:::

### As a Flake

```nix [flake.nix]
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    colmena.url = "github:zhaofengli/colmena"; # [!code --]
    wire.url = "github:mrshmllow/wire/stable"; # [!code ++]
  };
  outputs =
    { nixpkgs, colmena, ... }:
    {
      colmenaHive = colmena.lib.makeHive { # [!code --]
      wire = colmena.lib.makeHive { # [!code ++]
        # ..
      };
    };
}
```

### Non-flake

::: tip
You should also follow [installation](/guides/installation) to setup
npins and install the binary.
:::

Unlike colmena, you must call `makeHive` directly even in non-flake hives.

```nix [hive.nix]
let
  sources = import ./npins;
  wire = import sources.wire;
in
{ # [!code --]
wire.makeHive { # [!code ++]

  meta.nixpkgs = <nixpkgs>; # [!code --]
  meta.nixpkgs = import sources.nixpkgs { }; # [!code ++]

  # ...
}
```

Replacing `<nixpkgs>` with a pinned source is optional, but you should
probably use one if you ask me \:)

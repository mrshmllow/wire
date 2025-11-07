---
comment: true
title: Install wire
description: How to install wire tool.
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

::: info

The `wire` binary and the `wire.makeHive` function are tightly coupled, so it is
recommended that you use the same version for both.

:::

## Binary Cache

You should trust the substituter `https://wires.cachix.org` by
either editing `/etc/nix/nix.conf` or updating your NixOS configuration:

::: code-group

<<< @/snippets/getting-started/nix.conf
<<< @/snippets/getting-started/cache.nix [configuration.nix]

:::

## Installation through flakes

When using flakes, you should install wire through the same input you create
your hive from, sourced from the `stable` branch.

::: code-group
<<< @/snippets/guides/installation/flake.nix{38} [flake.nix]
:::

## Installation through npins

With npins you may allow it to use release tags instead of the `stable`
branch.

```sh
$ npins add github mrshmllow wire
```

::: code-group
<<< @/snippets/guides/installation/shell.nix{8} [shell.nix]
<<< @/snippets/guides/installation/hive.nix{8} [hive.nix]
:::

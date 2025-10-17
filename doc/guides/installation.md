---
comment: true
title: Install wire
description: How to install wire tool.
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

It is recommended you stick to versioned tags, the latest is `0.5.0`.

## Binary Cache

You should trust the substituter `https://wires.cachix.org` by
either editing `/etc/nix/nix.conf` or updating your NixOS configuration:

::: code-group

<<< @/snippets/getting-started/nix.conf
<<< @/snippets/getting-started/cache.nix [configuration.nix]

:::

## Installation through flakes

When using flakes, you should install wire through the same input you create
your hive from.

::: code-group
<<< @/snippets/guides/installation/flake.nix{38} [flake.nix]
:::

## Installation through npins

```sh
$ npins add github mrshmllow wire --at v0.5.0
```

::: code-group
<<< @/snippets/guides/installation/shell.nix{8} [shell.nix]
:::

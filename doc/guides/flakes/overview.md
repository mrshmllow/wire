---
comment: true
title: Use Flakes
description: How to output a hive from a flake.
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

## Output a hive

::: tip
If you have skipped ahead, please read the previous page to understand the
concept of a hive.
:::

You can use wire with a flake by outputting a hive with the `wire` flake output.
Just like when using a `hive.nix`, you must provide `meta.nixpkgs` which will
come from an input.

::: code-group
<<< @/snippets/getting-started/flake.nix [flake.nix]
:::

```
❯ nix flake show
git+file:///some/path
└───colmena: unknown
```

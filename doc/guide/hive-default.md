---
comment: true
title: hive.default
description: Deduplicate options with default node configuration.
---

# `{{ $frontmatter.title }}`

{{ $frontmatter.description }}

## Introduction

At the top level of a hive wire reserves the `defaults` attribute. Its applied
to every node.

::: warning

`defaults` must not rely on modules that a node imports, but a
node may rely on modules that default imports.

:::

```nix:line-numbers [hive.nix]
{
  meta.nixpkgs = import <nixpkgs> {};

  defaults = {
    # name of the node that defaults is being applied to
    name,
    # attribute set of all nodes
    nodes,
    ...
  }: {
    import = [
      ./default-module.nix

      # module that is imported for all nodes
      some-flake.nixosModules.default
    ];

    # default configuration
    # may or may not utilise `name` or `nodes`
  };

  node-a = {
    # some config
  };

  node-b = {
    # some more config
  };
}
```

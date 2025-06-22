---
comment: true
title: Meta Options
description: Wire hive meta options.
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

## meta.nixpkgs

Tells wire how to get `nixpkgs`.

_Type:_ A path or an instance of `nixpkgs`.

_Default:_ `inputs.nixpkgs.outPath`

_Examples:_

```nix
{
  # all valid options

  meta.nixpkgs = <nixpkgs>;

  meta.nixpkgs = import <nixpkgs> { };

  meta.nixpkgs = import sources.nixpkgs { };

  meta.nixpkgs = inputs.nixpkgs.outPath;

  meta.nixpkgs = inputs.other-nixpkgs.outPath;
}
```

## meta.specialArgs

Extra `specialArgs` to pass to each node & `default`.

::: tip

Wire always passes `name` (name of the node)
and `nodes` (attribute set of all nodes) as args, even if `meta.specialArgs =
{ }`.

:::

_Type:_ attribute set

_Default:_ `{ }`

_Example:_

```nix
{
  meta.specialArgs = {
    # pass flake inputs as specialArgs
    inherit inputs;
  };
}
```

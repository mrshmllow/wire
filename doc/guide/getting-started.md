---
comment: true
title: Getting Started
description: Getting started with Wire Tool!
---

# {{ $frontmatter.title }}

## Installation

Wire can be heavy to compile. You should enable the substituter `wires.cachix.org`.

::: code-group

<<< @/snippets/getting-started/cache.nix [module.nix]
<<< @/snippets/getting-started/nix.conf

:::

### Supported Nix & NixOS versions

Wire is currently _tested_ against `unstable`, `24.11` and `24.05`. It is only
tested against lix due to regressions with nix 2.26+!

### NixOS / Home Manager

::: code-group

<<< @/snippets/getting-started/nixos.flake.nix [flake.nix (NixOS)]
<<< @/snippets/getting-started/hm.flake.nix [flake.nix (Home Manager)]
<<< @/snippets/getting-started/configuration.nix
<<< @/snippets/getting-started/home.nix

:::

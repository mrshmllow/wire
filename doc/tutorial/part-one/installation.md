---
comment: true
title: Installation
description: Installing npins, wire, and enabling the binary cache.
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

::: warning
This page is for the purposes for the **Tutorial**.
You should read [Guides - Installation](/guide/installation.html) for installing Wire for
regular use.
:::

## Nix Installation

You should install nix if you do not have it on your system already.
There are detailed steps to installing Nix on [nix.dev](https://nix.dev/install-nix).

By the end of the installation, you should see something like this:

```sh
$ nix --version
nix (Nix) 2.11.0
```

## Using `cache.althaea.zone`

Because Wire can be heavy to compile, it is distributed with a [binary
cache](https://wiki.nixos.org/wiki/Binary_Cache). It's URL is
`https://cache.althaea.zone` and it's public key is
`cache.althaea.zone:BelRpa863X9q3Y+AOnl5SM7QFzre3qb+5I7g2s/mqHI=`.

You should trust the substituter `https://wires.cachix.org` by
either editing `/etc/nix/nix.conf` or updating your NixOS configuration:

::: code-group

<<< @/snippets/getting-started/nix.conf
<<< @/snippets/getting-started/cache.nix [configuration.nix]

:::

## Installing Wire

For the purposes of this tutorial we won't be directly installing wire, we'll
setup a nix development shell in the next stage.

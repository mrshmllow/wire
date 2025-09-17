---
comment: true
title: Preparing Repo & Shell
description:
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

## Initialising with Git & `npins`

First, lets create an adhoc shell to bring these two tools into our $PATH.

```sh
$ nix-shell -p git npins
$ git --version
git version 2.51.0
$ npins --version
npins 0.3.1
```

Great! Now lets use Git & `npins` to create a new Git repo and initialise it.
`npins init` may take a while to download `nixpkgs`.

```sh
$ git init -b main wire-tutorial
Initialized empty Git repository in /home/.../wire-tutorial/.git/
$ cd wire-tutorial/
$ npins init
[INFO ] Welcome to npins!
[INFO ] Creating `npins` directory
[INFO ] Writing default.nix
[INFO ] Writing initial lock file with nixpkgs entry (need to fetch latest commit first)
[INFO ] Successfully written initial files to 'npins/sources.json'.
```

This has created a pinned version of `nixpkgs` for us to use in our Wire hive.

## Adding wire as a dependency

We can now need to tell `npins` to use `wires-org/wire` as a dependency.

```sh
$ npins add github wires-org wire
[INFO ] Adding 'wire' …
    repository: https://github.com/wires-org/wire.git
    pre_releases: false
    submodules: false
    version: v0.4.0
    revision: f33d80c15b17c85d557d533441609a59a2210941
    hash: 0wgah341hvjpvppkgwjrj50rvzf56ccmjz720xsl3mw38h9nn6sr
    frozen: false
```

Great, now lets confirm the two dependencies we have added to this `npins`
project:

```sh
$ npins show
nixpkgs: (Nix channel)
    name: nixpkgs-unstable
    url: https://releases.nixos.org/nixpkgs/nixpkgs-25.11pre861972.88cef159e47c/nixexprs.tar.xz
    hash: 0zscvr0qa3capyzhmp798hgncz2qy8ggm843y10wk35jk7p0174f
    frozen: false

wire: (git release tag)
    repository: https://github.com/wires-org/wire.git
    pre_releases: false
    submodules: false
    version: v0.4.0
    revision: f33d80c15b17c85d557d533441609a59a2210941
    hash: 0wgah341hvjpvppkgwjrj50rvzf56ccmjz720xsl3mw38h9nn6sr
    frozen: false
```

## Creating a `shell.nix`

Open a text editor to edit `shell.nix` in the `wire-tutorial` directory.

```nix:line-numbers [shell.nix]
let
  sources = import ./npins;
  pkgs = import sources.nixpkgs { };
  wire = import sources.wire;
in
pkgs.mkShell {
  packages = [
    wire.packages.x86_64-linux.wire-small
    pkgs.npins
    pkgs.git
  ];

  NIX_PATH = "nixpkgs=${sources.nixpkgs.outPath}";
}
```

You should now `exit` to quit the old shell, and
enter a new shell with `nix-shell`. Since we added wire as a package, our new
shell should have wire in the $PATH:

```sh
$ exit
exit
$ nix-shell
$ wire --version
wire 0.5.0
Debug: Hive::SCHEMA_VERSION 0

```

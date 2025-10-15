---
comment: true
title: How-to Keep Using nixos-rebuild
description: How to combine outputs.nixosConfigurations with outputs.wire
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

## An Example

You can provide `makeHive` with your `nixosConfigurations` with the `inherit`
nix keyword. `makeHive` will merge any nodes and nixosConfigurations that share
the same name together.

::: tip
It should be noted that there are a few downsides. For example, you cannot access `config.deployment` from `nixosConfigurations`. For this reason it would be best practice to limit configuration in `colmena` to simply defining keys and deployment options.
:::

::: code-group
<<< @/snippets/getting-started/flake-merged.nix [flake.nix]
:::

Now, if we run `wire show`, you will see that wire only finds
the `nixosConfigurations`-es that also match a node in the hive.

```
❯ nix run ~/Projects/wire#wire-small -- show
Hive {
    nodes: {
        Name(
            "node-a",
        ): Node {
            target: Target {
                hosts: [
                    "node-a",
                ],
                user: "root",
                port: 22,
                current_host: 0,
            },
            build_remotely: false,
            allow_local_deployment: true,
            tags: {},
            keys: [],
            host_platform: "x86_64-linux",
        },
    },
    schema: 0,
}
```

This way, you can continue using `nixos-rebuild` and wire at the same time.

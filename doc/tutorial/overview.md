---
comment: false
title: Wire Tutorial Overview
description: In this tutorial we will create and deploy a Wire Hive.
---

# {{ $frontmatter.title }}

Wire is a tool to deploy NixOS systems. Its usage is inspired by [colmena](https://colmena.cli.rs/). In many places it's configuration attempts to remain a superset[^1] of colmena, however it is **not** a fork.

[^1]: A lot of your colmena module options will continue to work with wire, but wire has additional ergonomic changes you can take advantage of.

::: warning
Wire is alpha software, please use at your own risk. Many features listed in this documentation overall may not be complete / implemented, however features covered in this this tutorial are considered complete.
:::

---

In this tutorial we will create and deploy a Wire Hive. Along the way we will
encounter [npins](https://github.com/andir/npins), simple NixOS
configurations, virutal machines, and deployment keys.

<div class="tip custom-block" style="padding-top: 8px">

Ready? Skip to [Installation](./part-one/installation).

</div>

## Why Wire?

| Features                 | Wire               | Colmena            |
| ------------------------ | ------------------ | ------------------ |
| Node Tagging             | :white_check_mark: | :white_check_mark: |
| Secret Management        | :white_check_mark: | :white_check_mark: |
| Parallel Evaluation      | :white_check_mark: | :white_check_mark: |
| Node Tagging             | :white_check_mark: | :white_check_mark: |
| Remote Builds            | :white_check_mark: | :white_check_mark: |
| Pipeline Support         | :white_check_mark: | :x:[^2]            |
| Non-Root Deployments[^4] | :white_check_mark: | :x:[^3]            |

[^2]: You need to write custom nix code to use Colmena hive metadata inside environments like CI pipelines, bash scripting, etc., which requires a knowledge of its internals. Recently it agained the [eval feature](https://colmena.cli.rs/unstable/features/eval.html) which has improved the situation since wire was first started.

[^3]: See https://github.com/zhaofengli/colmena/issues/120

[^4]:
    You may deploy with _any_ user who can login through SSH, whether they be
    `wheel` or not. You may need to enter your password multiple times for the various elevated
    steps wire needs to perform.

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
configurations, and create deployment keys.

<div class="tip custom-block" style="padding-top: 8px">

Ready? Skip to [Installation](./installation).

</div>

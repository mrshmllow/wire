# Terminology

## Node

A node is a single computer, server, or otherwise machine that is running NixOS, uniquely identified by its name. A node may have associated tags, and must have deployment options.

## Hive

A hive describes a set of nodes, default options that are applied to all nodes, and wire-tool specific meta configuration.

A hive is a `attrset`, in a `hive.nix` or outputted by a flake.

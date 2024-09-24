# Tagging

You can assign tags to nodes, and target nodes by tags. Use `--on @TAG` to reference them.

```nix
{
  node-a = {
    deployment = {
      tags = ["arm" "native"];
    };
  };
}
```

```
wire apply --on @arm
```

---
comment: true
title: Use a non-root user
description: Deploy as any user with wire.
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

## Deploying User Requirements

If your selected deployment user does not fit the following requirements, the
deployment commands will likely fail with an error:

|                                    | Password-based SSH | Non-interactive SSH Auth |
| :--------------------------------- | -----------------: | -----------------------: |
| In `wheel` (Sudo User)             |   ❌ Not Supported |             ✅ Supported |
| Not In `wheel` (Unprivileged user) |   ❌ Not Supported |         ❌ Not Supported |

- "In `wheel`" here meaning a sudoer, whether it be `root` or not.
- "Non-interactive SSH Auth" here most likely meaning an SSH key, anything that
  does not require keyboard input in the terminal.

## Changing the user

By default, the target is set to root:

```nix
{
  deployment.target.user = "root";
}
```

But it can be any user you want so long as it fits the requirements above.

```nix
{
  deployment.target.user = "root"; # [!code --]
  deployment.target.user = "deploy-user"; # [!code ++]
}
```

After this change, wire will prompt you for sudo authentication, and tell you
the exact command wire wants privileged:

```sh{6}
$ wire apply keys --on media
 INFO eval_hive: evaluating hive Flake("/path/to/hive")
...
 INFO media | step="Upload key @ NoFilter" progress="3/4"
deploy-user@node:22 | Authenticate for "sudo /nix/store/.../bin/key_agent":
[sudo] password for deploy-user:
```

## Using alternative privilege escalation

You may change the privilege escalation command with the
[deployment.privilegeEscalationCommand](/reference/module.html#deployment-privilegeescalationcommand)
option.

For example, doas:

```nix
{
  deployment.privilegeEscalationCommand = [
    "sudo" # [!code --]
    "--" # [!code --]
    "doas" # [!code ++]
  ];
}
```

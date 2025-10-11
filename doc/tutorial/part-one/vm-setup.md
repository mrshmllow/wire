---
comment: true
title: Creating a Virtual Machine
description:
---

# {{ $frontmatter.title }}

{{ $frontmatter.description }}

## Creating a `vm.nix`

Open a text editor and edit `vm.nix`. Place in it this basic NixOS
virtual machine configuration, which enables openssh and forwards it's 22 port.

```nix:line-numbers [vm.nix]
let
  sources = import ./npins;
in
{
  imports = [ "${sources.nixpkgs}/nixos/modules/virtualisation/qemu-vm.nix" ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  networking.hostName = "wire-tutorial";

  # enable openssh
  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "yes";
  };

  virtualisation = {
    useBootLoader = true;

    # forward `openssh` port 22 to localhost:2222.
    forwardPorts = [
      {
        from = "host";
        host.port = 2222;
        guest.port = 22;
      }
    ];
  };

  # set a password and autologin
  services.getty.autologinUser = "root";
  users.users.root.initialPassword = "root";

  system.stateVersion = "23.11";
}
```

## Building & Running the virtual machine

Open a seperate Terminal tab/window/instance, ensuring you enter the development
shell with `nix-shell`.
Then, build the virtual machine with a bootloader,
taking our `vm.nix` as the nixos configuration.

```sh
$ nix-shell
[nix-shell:~/scratch/wire-tutorial]$ nix-build '<nixpkgs/nixos>' -A vmWithBootLoader -I nixos-config=./vm.nix
```

Building the virtual machine can take some time, but once it completes, start it
by running:

```sh
[nix-shell:~/scratch/wire-tutorial]$ ./result/bin/run-wire-tutorial-vm
```

You will see boot-up logs fly across the screen and eventually you will be placed
into shell inside the virtual machine.

```sh
running activation script...
setting up /etc...

Welcome to NixOS 25.11 (Xantusia)!

[  OK  ] Created slice Slice /system/getty.
[  OK  ] Created slice Slice /system/modprobe.
...
<<< Welcome to NixOS 25.11pre861972.88cef159e47c (x86_64) - hvc0 >>>

Run 'nixos-help' for the NixOS manual.

wire-tutorial login: root (automatic login)

[root@wire-tutorial:~]#

```

::: details
Further details on how the above commands work can be found at
[nix.dev](https://nix.dev/tutorials/nixos/nixos-configuration-on-vm.html#creating-a-qemu-based-virtual-machine-from-a-nixos-configuration)
:::

## Summary

Congratulations, you created a virtual machine in your terminal.
We'll be deploying to this virtual machine, so keep the
window open.

If you ever want to quit the virtual machine, run the command `poweroff`.

# wire docs

`reference/{cli,module}.nix` are filled in during a nix build w/ nixos module docs generation and clap. Read `package.nix`'s patchPhase for the details.

## Develop

```sh
pnpm install
pnpm run dev
```

## Build

```sh
nix build .#docs

# or

pnpm install
pnpm run build
```

The build will be found in `result/` or `.vitepress/dist` respectively.

import { defineConfig } from "vitepress";
import pkg from "../package.json";
import markdownItFootnote from "markdown-it-footnote";
import { withMermaid } from "vitepress-plugin-mermaid";
import {
  groupIconMdPlugin,
  groupIconVitePlugin,
  localIconLoader,
} from "vitepress-plugin-group-icons";

// https://vitepress.dev/reference/site-config
export default withMermaid(
  defineConfig({
    title: "wire",
    description: "a tool to deploy nixos systems",
    themeConfig: {
      search: {
        provider: "local",
      },

      footer: {
        message:
          'Released under the <a href="https://github.com/wires-org/wire/blob/main/COPYING">AGPL-3.0 License</a>.',
        copyright: "Copyright 2024-2025 wire Contributors",
      },

      // https://vitepress.dev/reference/default-theme-config
      nav: [
        { text: "Home", link: "/" },
        { text: "Guide", link: "/guide/wire" },
        { text: "Reference", link: "/reference/cli" },
        {
          text: pkg.version,
          items: [
            {
              text: "Changelog",
              link: "https://github.com/wires-org/wire/blob/main/CHANGELOG.md",
            },
          ],
        },
      ],

      sidebar: {
        "/guide/": [
          {
            text: "Introduction",
            items: [
              { text: "What is Wire?", link: "/guide/wire" },
              { text: "Getting Started", link: "/guide/getting-started" },
              { text: "Flakes", link: "/guide/flakes" },
              { text: "Applying Your Config", link: "/guide/apply" },
              { text: "Targeting Nodes", link: "/guide/targeting" },
            ],
          },
          {
            text: "Features",
            items: [
              { text: "Secret management", link: "/guide/keys" },
              { text: "Parallelism", link: "/guide/parallelism" },
              { text: "hive.default", link: "/guide/hive-default" },
              { text: "Magic Rollback", link: "/guide/magic-rollback" },
            ],
          },
          {
            text: "Use cases",
            items: [{ text: "Tailscale", link: "/guide/tailscale" }],
          },
        ],
        "/reference/": [
          {
            text: "Reference",
            items: [
              { text: "CLI", link: "/reference/cli" },
              { text: "Meta Options", link: "/reference/meta" },
              { text: "Module Options", link: "/reference/module" },
              { text: "Error Codes", link: "/reference/errors" },
            ],
          },
        ],
      },

      editLink: {
        pattern: "https://github.com/wires-org/wire/edit/main/doc/:path",
        text: "Edit this page on GitHub",
      },

      socialLinks: [
        { icon: "github", link: "https://github.com/wires-org/wire" },
      ],
    },
    markdown: {
      config: (md) => {
        md.use(markdownItFootnote);
        md.use(groupIconMdPlugin);
      },
    },
    vite: {
      // https://github.com/mermaid-js/mermaid/issues/4320#issuecomment-1653050539
      optimizeDeps: {
        include: ["mermaid"],
      },
      plugins: [
        groupIconVitePlugin({
          customIcon: {
            nixos: "vscode-icons:file-type-nix",
            "configuration.nix": "vscode-icons:file-type-nix",
            "hive.nix": "vscode-icons:file-type-nix",
            "flake.nix": "vscode-icons:file-type-nix",
            "module.nix": "vscode-icons:file-type-nix",
            home: localIconLoader(import.meta.url, "../assets/homemanager.svg"),
            ".conf": "vscode-icons:file-type-config",
          },
        }),
      ],
    },
  }),
);

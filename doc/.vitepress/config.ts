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
        { text: "Tutorial", link: "/tutorial/overview" },
        { text: "Guides", link: "/guides/installation" },
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
        "/": [
          {
            text: "Tutorial",
            collapsed: false,
            items: [
              { text: "Overview", link: "/tutorial/overview" },
              {
                text: "Part One",
                items: [
                  {
                    text: "Installation",
                    link: "/tutorial/part-one/installation",
                  },
                  {
                    text: "Preparing Repo & Shell",
                    link: "/tutorial/part-one/repo-setup",
                  },
                  {
                    text: "Creating a Virtual Machine",
                    link: "/tutorial/part-one/vm-setup",
                  },
                  {
                    text: "Basic Hive & Deployment",
                    link: "/tutorial/part-one/basic-hive",
                  },
                ],
              },
              {
                text: "Part Two",
                items: [
                  {
                    text: "Basic Deployment Keys",
                    link: "/tutorial/part-two/basic-keys",
                  },
                  {
                    text: "Encrypted Deployment Keys",
                    link: "/tutorial/part-two/encryption",
                  },
                ],
              },
            ],
          },
          {
            text: "How-to Guides",
            collapsed: false,
            items: [
              { text: "Installing Wire", link: "/guides/installation" },
              { text: "Applying Your Config", link: "/guides/apply" },
              { text: "Targeting Nodes", link: "/guides/targeting" },
              {
                text: "Flakes",
                items: [
                  { text: "Overview", link: "/guides/flakes/overview" },
                  {
                    text: "How-to Keep Using nixos-rebuild",
                    link: "/guides/flakes/nixos-rebuild",
                  },
                ],
              },
              {
                text: "Features",
                items: [
                  { text: "Secret management", link: "/guides/keys" },
                  { text: "Parallelism", link: "/guides/parallelism" },
                  { text: "hive.default", link: "/guides/hive-default" },
                ],
              },
            ],
          },
          { text: "CLI & Module Reference", link: "/reference/cli.html" },
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
        md.use(groupIconMdPlugin, {
          titleBar: { includeSnippet: true },
        });
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
            "vm.nix": "vscode-icons:file-type-nix",
            "shell.nix": "vscode-icons:file-type-nix",
            "secrets.nix": "vscode-icons:file-type-nix",
            home: localIconLoader(import.meta.url, "../assets/homemanager.svg"),
            ".conf": "vscode-icons:file-type-config",
          },
        }),
      ],
    },
  }),
);

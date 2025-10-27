---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
  name: wire
  text: a tool to deploy nixos systems
  # tagline: My great project tagline
  actions:
    - theme: brand
      text: Read Tutorial
      link: /tutorial/overview
    - theme: alt
      text: How-to Guides
      link: /guides/installation
    - theme: alt
      text: Sources
      link: https://github.com/mrshmllow/wire.git

features:
  - title: Deploy in Parallel
    details: Build and deploy many nodes at once.
    icon: 💽
  - title: Manage Secrets
    details: Fast & Unopinionated secret management. Bring your own GPG, Age, or any other encryption tool.
    icon: 🔑
  - title: Deploy as Any User
    details: Non-root deployments and interactive authentication is fully supported.
    icon: 🧑‍💻
---

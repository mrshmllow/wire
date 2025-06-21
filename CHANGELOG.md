# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - yyyy-mm-dd

### Added

- Nodes may now fail without stopping the entire hive from continuing. A summary
  of errors will be presented at the end of the apply process.
- Wire will now ping the node before it proceeds executing.
- Wire will now properly respect `deployment.target.hosts`.
- Wire will now attempt each target host in order until a valid one is found.

## [0.3.0] - 2025-06-20

### Added

- Run tests against `unstable` and `25.05` by @mrshmllow in https://github.com/wires-org/wire/pull/176.

### Changed

- Dependency Updates.
- Wire now compiles and includes key agents for multiple architectures, currently only linux.
- There is a new package output, `wire-small`, for testing purposes.
  It only compiles the key agent for the host that builds `wire-small`.
- `--no-progress` now defaults to true if stdin does not refer to a tty (unix pipelines, in CI).
- Added an error for the internal hive evluation parse failure.
- The `inspect` command now has `show` as an alias.
- Remove `log` command as there are currently no plans to implement the feature
- The `completions` command is now hidden from the help page

### Fixed

- A non-existant key owner user/group would not default to gid/uid `0`.
- Keys can now be deployed to localhost.

## [0.2.0] - 2025-04-21

### Added

- Getting Started Guide by @mrshmllow.
- Web documentation for various features by @mrshmllow.
- Initial NixOS VM Testing Framework by @itslychee in https://github.com/wires-org/wire/pull/93.

### Changed

- `runtime/evaluate.nix`: force system to be null by @itslychee in https://github.com/wires-org/wire/pull/84.

> [!IMPORTANT]  
> You will have to update your nodes to include `nixpkgs.hostPlatform = "<ARCH>";`

- GH Workflows, Formatting, and other DevOps yak shaving.
- Issue Templates.
- Cargo Dependency Updates.
- `doc/` Dependency Updates.
- `flake.nix` Input Updates.

### Fixed

- Keys with a path source will now be correctly parsed as `path` instead
  of `string` by @mrshmllow in https://github.com/wires-org/wire/pull/131.
- `deployment.keys.<name>.destDir` will be automatically created if it
  does not exist. Nothing about it other than existence is guaranteed. By
  @mrshmllow in https://github.com/wires-org/wire/pull/131.

# Changelog

All notable changes to the `aptos-move-flow` crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2026-06-30

### Added
- `move-flow update [--check]` subcommand for self-updating from
  `aptos-labs/aptos-ai` GitHub releases.

## [1.0.4] - 2026-06-10

### Added
- Cross-platform GitHub Releases pipeline driven by the
  `.github/workflows/move-flow-release.yaml` workflow, producing prebuilt
  archives for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
  `x86_64-apple-darwin`, `aarch64-apple-darwin`, and
  `x86_64-pc-windows-msvc`.

[Unreleased]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v1.1.0...HEAD
[1.1.0]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v1.0.4...move-flow-v1.1.0
[1.0.4]: https://github.com/aptos-labs/aptos-core/releases/tag/move-flow-v1.0.4

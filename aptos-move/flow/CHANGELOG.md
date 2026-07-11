# Changelog

All notable changes to the `aptos-move-flow` crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.0] - 2026-07-11

### Changed
- **Breaking:** `facts` queries report function returns as `returnTypes`, an
  array with one entry per tuple element, instead of the optional
  `returnType` display string.
- **Breaking:** struct types in function signatures, struct fields,
  `resourceAccess`, and cross-module references are fully qualified
  (`address::module::Name`) in `facts` output.
- Package build failures in `move_package_query` return `invalid_params`
  instead of internal errors.

### Added
- Attribute arguments and assignment values are preserved in `facts` output
  (previously only attribute names were serialized).
- Compiler-synthesized lambda-lifted functions are tagged with
  `isLambdaLifted` and linked to their defining function via `definedIn`;
  `module_summary` carries the same flag.
- `move_package_query` is annotated read-only in its MCP tool annotations.

### Fixed
- The `facts` path is guarded against panics, matching `function_usage`.

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

[Unreleased]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v2.0.0...HEAD
[2.0.0]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v1.1.0...move-flow-v2.0.0
[1.1.0]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v1.0.4...move-flow-v1.1.0
[1.0.4]: https://github.com/aptos-labs/aptos-core/releases/tag/move-flow-v1.0.4

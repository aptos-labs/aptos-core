# Changelog

All notable changes to the `aptos-move-flow` crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.4] - 2026-06-10

### Added
- Cross-platform GitHub Releases pipeline driven by the
  `.github/workflows/move-flow-release.yaml` workflow, producing prebuilt
  archives for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
  `x86_64-apple-darwin`, `aarch64-apple-darwin`, and
  `x86_64-pc-windows-msvc`.
- Per-artifact `.sha256` files and an aggregated `SHA256SUMS` manifest
  published alongside each release for integrity verification.
- Pre-flight safety gates in the release workflow:
  - Version-consistency check that fails the build when the workflow's
    `release_version` input does not match the `version` in
    `aptos-move/flow/Cargo.toml`.
  - Tag-collision check that fails fast when
    `move-flow-v<release_version>` already exists as a tag or GitHub
    release, before any platform build runs.
  - `dry_run` mode that exercises the full build/package path without
    publishing artifacts.
  - Source-SHA pinning: the pre-flight job resolves the requested ref
    (branch, tag, or SHA) to a single commit and pins every platform
    build to it, so a branch that moves mid-run cannot split the
    release across commits.

## [1.0.3]

Initial development releases prior to the hardened release pipeline.
Release history before 1.0.4 was not tracked in this changelog; see
`git log -- aptos-move/flow` for details.

[Unreleased]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v1.0.4...HEAD
[1.0.4]: https://github.com/aptos-labs/aptos-core/releases/tag/move-flow-v1.0.4
[1.0.3]: https://github.com/aptos-labs/aptos-core/commits/main/aptos-move/flow

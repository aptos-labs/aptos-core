# Changelog

All notable changes to the `aptos-move-flow` crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.1.0] - 2026-09-21

### Added
- `move_spec_check` MCP tool: runs every acceptance check for a specification
  task (compilation, complete target verification, contract-category coverage,
  forbidden weakening) and, given a baseline package, reports whether the
  runtime implementation or the edit scope changed. Returns
  `CANDIDATE_ACCEPTED` when the task is done.
- Specification-inference tactics for `/move-inf`: `hybrid-guided` (default),
  `hybrid-flexible`, and `agent-only`. Hybrid plugins pick the tactic per
  invocation; `--inference-tactic` or `MOVE_FLOW_INFERENCE_TACTIC` sets the
  default, and `agent-only` generates a plugin without the WP tool.
- `move-flow experiment` subcommands (`inventory`, `check-package`, `infer`,
  `prove`, `check-candidate`, `compare-implementation`) with JSONL telemetry.
- Plugin generator options `--evaluation-mode`, `--feedback-level`,
  `--no-wp-simplification`, `--telemetry-jsonl`, and `--flow-source-commit`.
  The generated `move-flow-manifest.json` records the tactic, evaluation flag,
  rendered inference-skill hash, and MCP tool-inventory hash.
- `move_package_verify` and `move_package_wp` accept
  `address::module_name::function_name` filters (numeric or named address);
  an unqualified module name must be unambiguous.
- `move_package_verify` reports loop-invariant evidence and per-obligation
  timeout attribution; the verification skills cover quantifier-instantiation
  replays, lemmas, proof hints, triggers, and `[weight = N]`.
- Specification-inference evaluation study (corpus, sandboxed harness, scorer)
  under `evaluation/spec-inference`.

### Changed
- `move_package_verify` runs one bounded Boogie process per verification root
  and no longer applies `backend.shards` from `Prover.toml`.
- `move_package_verify` enforces a fixed aggregate diagnostic limit in addition
  to `error_limit`; `error_limit` must be in `[1, 20]` and `timeout` in
  `[1, 60]`.
- WP inference handles abort paths, guarded loops, ghost state, map membership,
  companion signatures, vector intrinsics, and sequential call pre-states.
- Generated hook commands receive the session environment and forward hook
  input with `printf` instead of `echo`.
- In an evaluation session the replay tool is not served and manifests naming
  remote (`git` or on-chain) dependencies are refused before any build.

### Fixed
- `/move-inf` failed to register because a leading Tera control tag pushed the
  skill frontmatter off line 1; rendering now refuses any skill or agent whose
  output does not begin with its frontmatter.

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

[Unreleased]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v2.1.0...HEAD
[2.1.0]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v2.0.0...move-flow-v2.1.0
[2.0.0]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v1.1.0...move-flow-v2.0.0
[1.1.0]: https://github.com/aptos-labs/aptos-core/compare/move-flow-v1.0.4...move-flow-v1.1.0
[1.0.4]: https://github.com/aptos-labs/aptos-core/releases/tag/move-flow-v1.0.4

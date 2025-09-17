# AGENT.md

This file is the local coding-agent guide for `third_party/move/tools/move-fuzz/`.

Unless the task explicitly requires otherwise, keep changes focused on this subtree plus the small set of wrapper files that expose it:

- `third_party/move/tools/move-fuzz/`
- `aptos-move/cli/src/fuzz.rs`
- `third_party/move/tools/move-fuzz/README.md`

## What move-fuzz is now

move-fuzz is no longer just a script generator. The current `auto` pipeline does all of the following:

1. Resolves the Move project and its package graph.
2. Builds primary packages, dependency packages, and framework packages.
3. Performs static analysis over types and callable functions.
4. Generates fuzz driver scripts.
5. Compiles generated entrypoints.
6. Runs a two-phase fuzzing campaign.

Phase 1 is single-transaction fuzzing with online profiling.
Phase 2 is multi-transaction fuzzing driven by the DUG and chain seed pools.

The design inspiration is in `docs/idea.pdf`, but the implementation intentionally goes beyond the slides. When changing DUG-related logic, compare against `docs/idea.pdf` and call out any semantic gap you introduce or close.

## Primary entrypoints and files

### User-facing entrypoints

- `src/cli.rs`
  Core subcommand definitions and the main `run_on()` entrypoint.
- `src/bin/move-fuzz-dev.rs`
  Standalone developer runner with the same CLI shape as `aptos move fuzz`.
- `aptos-move/cli/src/fuzz.rs`
  Aptos CLI wrapper that exposes the fuzzer as `aptos move fuzz`.

### Core runtime

- `src/fuzzer.rs`
  Main orchestration for script generation, entrypoint caching, fuzz loop startup, phase transition, campaign persistence, and reporting.
- `src/executor/oneshot.rs`
  Phase 1 single-script fuzzers and per-entrypoint seed/corpus handling.
- `src/executor/sequence.rs`
  Phase 2 chain fuzzers, DUG logic, multi-transaction scheduling, seed-chain construction, and sequence database.
- `src/executor/tracing.rs`
  VM execution and execution-profile harvesting.
- `src/mutate/mutator.rs`
  Value generation and mutation logic.
- `src/state.rs`
  Persistent state, cache formats, and snapshot/load/save helpers.

### Static analysis and script generation

- `src/prep/typing.rs`
  Type model, simple-vs-complex classification, substitutions, and unification.
- `src/prep/datatype.rs`
  Datatype discovery and content analysis.
- `src/prep/function.rs`
  Public function discovery and registry.
- `src/prep/graph.rs`
  Flow-graph enumeration for satisfying complex arguments.
- `src/prep/model.rs`
  Drives graph generation, feasibility filtering, per-function script caps, and script-generation progress reporting.
- `src/prep/canvas.rs`
  Lowers feasible flow graphs into imperative driver scripts.

### Project and package handling

- `src/deps.rs`
  Package discovery, dependency resolution, named addresses, package classification.
- `src/package.rs`
  Build/test helpers and package-build cache support.

## Commands you should actually use

Run these from the repo root.

### Build

```bash
cargo build -p move-fuzz --bin move-fuzz-dev
cargo build -p aptos
```

Use the first one for fast iteration on the fuzzer itself. Use the second when you changed the Aptos CLI wrapper or want to validate the integrated binary.

### Test

```bash
cargo test -p move-fuzz --lib
cargo test -p move-fuzz --lib -- <test_name>
```

`--lib` is the normal fast path for this crate.

### Format

```bash
cargo +nightly fmt -p move-fuzz
```

This repo expects nightly `rustfmt` behavior.

### Representative local run

```bash
cargo run -p move-fuzz --bin move-fuzz-dev -- \
  /path/to/project \
  --in-place \
  --skip-deps-update \
  auto --dry-run
```

Use `--dry-run` when debugging package resolution, model building, script generation, and entrypoint compilation without entering the fuzz loop.

## Real CLI shape and important flags

The real shape is:

```bash
aptos move fuzz <PATH> [TOP_LEVEL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

or

```bash
move-fuzz-dev <PATH> [TOP_LEVEL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

Useful top-level flags:

- `--subdir <PATH>`
  Limit the package roots under a large workspace.
- `--in-place`
  Avoid the temporary workspace copy. Very useful on large projects and when debugging generated artifacts.
- `--skip-deps-update`
  Avoid dependency update churn during resolution.
- `-v`, `-vv`, `-vvv`
  Info, debug, and trace logging.

Important `auto` flags:

- `--seed`
- `--max-trace-depth`
- `--max-call-repetition`
- `--max-script-gen-secs-per-function`
- `--dry-run`
- `--state-dir`
- `--reset-state`
- `--max-chain-length`
- `--max-chain-repetition`
- `--saturation-secs`

## Current behavior that agents must not accidentally regress

### Package inclusion policy

For `auto`, the CLI forces both dependencies and framework packages to be visible during analysis.
That is intentional.
Do not “optimize” this away unless you also rework provider discovery and script-generation semantics.

### External provider policy

`PkgKind::Primary` and `PkgKind::Dependency` are valid external providers.
`PkgKind::Framework` is not.
Primary providers are preferred over dependency providers.
If you change provider discovery, preserve that policy unless the user explicitly asks for a broader search.

### Script-generation budgets

There are two separate controls:

- Graph-count budget in `prep/graph.rs`
  `MAX_DERIVED_GRAPHS_PER_PROCESS = 4096`
- Wall-clock budget in `auto`
  `--max-script-gen-secs-per-function`, default `600`, `0` disables the time cap

The graph-count budget is per function, not global. `GraphBuilder::process()` resets it per primary-function pass.

### Per-function script cap

`prep/model.rs` limits generated scripts with `MAX_SCRIPTS_PER_FUNCTION = 24`.
If a change increases script counts, validate that you are not just reintroducing near-duplicate wrappers.

### Incomplete graph handling

Graph exploration may stop early due to the graph budget or the wall-clock budget.
That means incomplete graphs can exist transiently during generation.

Current invariants:

- `GraphBuilder::is_feasible()` must reject graphs that do not provide every required complex argument.
- `DriverCanvas` must degrade safely for incomplete graphs.
- `DriverCanvas::try_build()` is the correct entrypoint for codegen.
- Do not reintroduce `expect()`-based panics for missing complex-type providers in the canvas path.

### Phase structure

Phase 1:

- oneshot fuzzers run per generated script
- execution profiles are collected online
- initial DUG/bootstrap data is accumulated during execution

Phase 2:

- chain fuzzers are created lazily after Phase 1 saturation
- scheduling becomes DUG- and chain-seed-driven
- stopping uses `saturation_secs` against Phase 2 novelty as well

If you change phase transitions, seed selection, or DUG growth, test both phases explicitly.

## State, cache, and resumability

The default state directory is `<project>/.move-fuzz` unless `--state-dir` is provided.

Important files/directories under that state dir:

- `auto_state.json`
  Persisted fuzz-loop state.
- `entrypoints_cache.json`
  Persisted compiled/generated entrypoints.
- `package-cache/`
  Persistent earlier-stage package build cache.
- `fuzz_stats.json`
  Frontend/progress reporting for long runs.

Important nuance:

- package builds are cached persistently
- generated entrypoints are cached persistently once successfully produced
- fuzz-loop state is resumable
- partial script-generation-in-progress is not resumable; interruption means regeneration from scratch until a complete entrypoint cache is produced

When the user wants a clean slate, use `--reset-state`.

## `fuzz_stats.json` is the first place to look

The stats file is the cheapest way to understand where a long run is spending time.

Typical stages include:

- `building_packages`
- `preparing_autogen`
- `script_generation`
- fuzz-loop stages reported by `fuzzer.rs`

When debugging large targets, inspect `fuzz_stats.json` before assuming the run is stuck.

## Expected validation after nontrivial changes

At minimum:

1. `cargo +nightly fmt -p move-fuzz`
2. `cargo test -p move-fuzz --lib`
3. `cargo build -p move-fuzz --bin move-fuzz-dev`

For changes affecting CLI integration, also build `aptos`.
For changes affecting generation, provider discovery, DUG logic, caching, or runtime behavior, run a fuzzing campaign on a representative Move contract.

## Documentation expectations

If you change CLI behavior, persistence behavior, phase behavior, or recommended invocation patterns, update:

- `README.md`
- this file (`AGENT.md`)

Keep the README user-facing and keep this file agent-facing.

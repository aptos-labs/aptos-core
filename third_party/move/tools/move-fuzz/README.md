# Move Fuzzer

This directory contains the source code of a coverage-guided fuzzer for Move smart contracts.

## Design

For the high-level design -- the DUG idea, the two-phase algorithm, the module
map, the persistence model, determinism, and known limitations -- see
[`docs/design.md`](docs/design.md). Read that before changing anything beyond the
CLI surface.

## File Layout

```txt
# Command-line interface (starting point of code logic)
- cli.rs

# Configurations and useful type definitions
- common.rs
- language.rs

# Accounts and address bookkeeping
- account.rs

# Package (including dependency) resolution, build, and testing
- deps.rs
- package.rs

# Local testnet (localnet) simulation, used by the `exec` subcommand
- simulator.rs
- testnet.rs

# Fuzzing core: campaign orchestration and persisted state
- fuzzer.rs
- state.rs

- prep/
  # Static analysis and driver-script generation
  - ident.rs
  - typing.rs
  - datatype.rs
  - function.rs
  - graph.rs
  - model.rs
  - canvas.rs

- mutate/
  # Input generation and mutation
  - mutator.rs

- executor/
  # Simulated execution and the two fuzzing phases
  - mod.rs       # coverage-map merge/count/delta helpers
  - tracing.rs   # forked FakeExecutor, read/write tracking, coverage tracing
  - oneshot.rs   # Phase 1: single-transaction fuzzers
  - sequence.rs  # Phase 2: DUG, chain construction, chain fuzzers

# Utilities not directly related to fuzzing
- utils.rs
- subexec.rs
```

## User Guide

### Build the fuzzer

The fuzzer is integrated into the `aptos` binary in the monorepo. Build it with:

```bash
cargo build -p aptos
```

For development, you can also build the standalone developer runner:

```bash
cargo build -p move-fuzz --bin move-fuzz-dev
```

`move-fuzz-dev` exposes the same `move-fuzz` CLI without the full Aptos CLI shell.

### CLI shape

The main user-facing command is:

```bash
aptos move fuzz [TOP_LEVEL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

The standalone developer runner uses the same arguments:

```bash
move-fuzz-dev [TOP_LEVEL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

The project root is selected with `--package-dir <PATH>`, the same flag every other
`aptos move` subcommand uses, and defaults to the current directory. Unlike the other
subcommands, it may point at a directory tree that holds several Move packages rather
than a single `Move.toml`.

### Top-level options

These options apply to all subcommands. Where a flag also exists on the other `aptos move`
subcommands it keeps the same name and value syntax (see `MovePackageOptions`); only the
defaults differ, as noted below.

- `--package-dir <PATH>`
  Root directory of the Move project collection to analyze. Defaults to the current directory.
- `--subdir <PATH>`
  Restrict the analysis to one or more package directories under `--package-dir`. Pass it multiple times to fuzz a subset of a large workspace.
- `--language-version <VERSION>` (alias: `--language`)
  Select the Move language version. Defaults to `2.3` (the other subcommands default to the latest stable version).
- `--optimize <none|default|extra>`
  Select the optimization level. Defaults to `extra`, so that fuzzing exercises the full optimizer pipeline.
- `--alias <NAME=NAME>`
  Declare named-address aliases.
- `--resource <RESOURCE=BASE:SEED>`
  Declare resource-account derivations.
- `--in-place`
  Run directly in the target directory instead of copying the project to a temporary working directory first. This is useful for large projects and for debugging generated artifacts in place.
- `--skip-deps-update`
  Skip automated dependency updates during project resolution. This is useful when the dependency state is already prepared and you want to avoid extra network or resolver churn.
- `-v`, `-vv`, `-vvv`
  Increase logging verbosity. `-v` enables info logs, `-vv` enables debug logs, and `-vvv` enables trace logs.

### Main fuzzing command

The main fuzzing entrypoint is:

```bash
aptos move fuzz <PATH> [TOP_LEVEL_OPTIONS] auto [AUTO_OPTIONS]
```

The `auto` command currently performs the full move-fuzz pipeline:

1. Resolve the project and relevant packages.
2. Build primary packages, dependencies, and framework packages.
3. Analyze datatypes and callable functions.
4. Generate driver scripts for fuzzable entrypoints.
5. Compile generated scripts.
6. Execute the fuzzing loop against a local simulated environment.

#### Important `auto` options

- `--seed <U64>`
  Seed all randomness used by the fuzzer.
- `--max-trace-depth <N>`
  Limit the depth of generated dependency traces.
- `--max-call-repetition <N>`
  Limit how many times a single function may appear in one generated trace.
- `--max-script-gen-secs-per-function <SECS>`
  Wall-clock budget for script generation per primary function. The default is `600` seconds. Set it to `0` to disable this time budget.
- `--num-user-accounts <N>`
  Number of user accounts to provision in the simulator.
- `--dry-run`
  Stop after script generation, without compiling the generated scripts and without entering the fuzzing loop. The generated `.move` sources are written to `<workdir>/autogen/sources/`; combine with `--in-place` to read them. Note that a dry run does not populate the entrypoint cache, so it does not speed up a later real run.
- `--string-dict <PATH>`
  Load an external string dictionary, one string per line.
- `--state-dir <PATH>`
  Store persistent state, caches, and stats in this directory. The default is `<project>/.move-fuzz`.
- `--reset-state`
  Wipe persistent move-fuzz state before starting. This removes cached package builds, cached entrypoints, seed/state files, and previous stats.
- `--max-chain-length <N>`
  Maximum dependency-chain length for multi-transaction fuzzing.
- `--max-chain-repetition <N>`
  Maximum number of times one script may repeat within a single chain.
- `--saturation-secs <SECS>`
  Seconds without new coverage before the fuzzer transitions from Phase 1 to Phase 2. Phase 1 also hands over unconditionally after 10x this value (or half of `--max-total-secs`, whichever is smaller), so one script that keeps finding coverage cannot keep the campaign out of Phase 2.
- `--max-total-secs <SECS>`
  Hard wall-clock budget for the whole campaign. The fuzzer writes a final checkpoint and exits when it elapses, in either phase. The budget is measured per invocation, so a run resumed from a checkpoint starts a fresh one. `0` (the default) means no limit.
- `--max-iterations <N>`
  Hard cap on mutation-loop rounds across both phases, counted per invocation, with the same clean shutdown. `0` (the default) means no limit.

#### Package filtering

The `auto`, `build`, and `test` subcommands support package filters:

- `--include-deps`
- `--include-framework`
- `--include-pkg <REGEX>`
- `--exclude-pkg <REGEX>`

For `auto`, move-fuzz forces `--include-deps` and `--include-framework` on internally, unless packages are explicitly excluded by name filters. This is intentional: script generation and fuzz execution need dependency and framework context.

### Persistent state and caches

By default, `auto` is resumable and persists state under `.move-fuzz/` in the project root.

That state currently includes:

- package build cache
- generated entrypoint cache
- fuzzing state
- stats output (`fuzz_stats.json`)

Use `--reset-state` when you want a clean-slate run.

### Periphery commands

#### List relevant Move packages

```bash
aptos move fuzz <PATH> [TOP_LEVEL_OPTIONS] list
```

#### Build relevant Move packages

```bash
aptos move fuzz <PATH> [TOP_LEVEL_OPTIONS] build [--dev] [FILTER_OPTIONS]
```

#### Run Move unit tests in relevant packages

```bash
aptos move fuzz <PATH> [TOP_LEVEL_OPTIONS] test [FILTER_OPTIONS] [--test-filter <NAME>] [--gas] [--single-thread]
```

#### Execute JSON runbooks on a fresh local simulator

```bash
aptos move fuzz <PATH> [TOP_LEVEL_OPTIONS] exec [--runbook <PATH>] [--realistic-gas]
```

### Common examples

#### Minimal fuzz run

```bash
aptos move fuzz /path/to/project auto
```

#### Dry-run script generation with verbose logs

```bash
aptos move fuzz /path/to/project -vv auto --dry-run --max-trace-depth 4 --max-call-repetition 2
```

#### Resume from a custom state directory

```bash
aptos move fuzz /path/to/project --state-dir /tmp/my-move-fuzz auto --seed 1
```

#### Clean-slate run

```bash
aptos move fuzz /path/to/project auto --reset-state
```


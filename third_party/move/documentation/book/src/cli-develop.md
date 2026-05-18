# Develop

Subcommands used during local development: scaffolding, building, testing, formatting, and inspecting bytecode. Most accept the [shared package options](./cli.md#package-options); only command-specific flags are listed below.

## `aptos move init`

Scaffold a new Move package: creates `Move.toml`, an empty `sources/` directory, and a `tests/` directory.

```shellscript filename="Terminal"
aptos move init --name my_package
aptos move init --name my_package --template hello-blockchain
```

| Flag | Meaning |
|---|---|
| `--name <NAME>` | Package name (required). |
| `--template <TEMPLATE>` | Pre-populate with a template, e.g., `hello-blockchain`. |
| `--package-dir <PATH>` | Where to create the package. Defaults to the current directory. |
| `--named-addresses <NAME=ADDR,...>` | Pre-fill named addresses in `Move.toml`. Use `_` as a placeholder for unassigned addresses. |

_See also: [Create Package](https://aptos.dev/build/smart-contracts/create-package), [Start a Move package from a template](https://aptos.dev/build/cli/start-from-template)._

## `aptos move compile` (alias: `build`)

Compile a Move package to bytecode, writing artifacts under `<package-dir>/build/`.

```shellscript filename="Terminal"
aptos move compile
aptos move compile --named-addresses example=0x42 --save-metadata
```

| Flag | Meaning |
|---|---|
| `--save-metadata` | Also write package metadata to `build/`. Required when constructing publish payloads manually. |
| `--included-artifacts <none\|sparse\|all>` | Which artifacts to embed. `none` = bytecode only; `sparse` (default) = enough to reconstruct sources; `all` = everything. Drives publishing gas cost. |
| `--fetch-deps-only` | Resolve and fetch dependencies, but skip the actual compile. |

_See also: [Compiling (Move)](https://aptos.dev/build/smart-contracts/compiling)._

## `aptos move compile-script` (alias: `build-script`)

Compile a single `script { ... }` source file into a transaction script blob and report its hash.

```shellscript filename="Terminal"
aptos move compile-script --package-dir my_script_pkg
```

The package must contain exactly one script source. Use the resulting `.mv` with [`aptos move run-script`](./cli-run.md#aptos-move-run-script).

_See also: [Compiling Move Scripts](https://aptos.dev/build/smart-contracts/scripts/compiling-scripts)._

## `aptos move test`

Run all functions annotated with `#[test]` in the package. Test code under `tests/` and inside `#[test_only]` modules is included automatically.

```shellscript filename="Terminal"
aptos move test
aptos move test --filter coin                 # only tests whose name contains "coin"
aptos move test --coverage                    # record line coverage to .coverage_map.mvcov
aptos move test --fail-fast                   # stop after first failing test
```

| Flag | Meaning |
|---|---|
| `--filter <STR>` / `-f <STR>` | Run only tests whose fully-qualified name contains `STR`. |
| `--coverage` | Record bytecode coverage. Pair with `aptos move coverage`. |
| `--instructions <N>` / `-i <N>` | Maximum instructions per test (default `100000`). |
| `--ignore-compile-warnings` | Don't fail if the build emits warnings. |
| `--dump` | Dump storage state on failure. |
| `--fail-fast` | Abort on first failure. |

## `aptos move coverage`

Inspect coverage data recorded by `test --coverage`. Three subcommands:

```shellscript filename="Terminal"
aptos move coverage summary             # per-module coverage percentage
aptos move coverage summary --summarize-functions
aptos move coverage source --module my_module
aptos move coverage bytecode --module my_module
```

| Subcommand | Output |
|---|---|
| `summary` | Per-module coverage. `--csv` switches to CSV; `--summarize-functions` adds per-function detail. |
| `source` | Annotates Move source with coverage indicators. |
| `bytecode` | Annotates a disassembled module with coverage indicators. |

## `aptos move prove`

Run the [Move Prover](./spec-lang.md) against the package's spec blocks.

The prover relies on two external binaries — `boogie` and `z3` — which the CLI doesn't bundle. Install (or update) them once before running `prove`:

```shellscript filename="Terminal"
aptos update prover-dependencies
```

Then:

```shellscript filename="Terminal"
aptos move prove
aptos move prove --filter Counter --vc-timeout 60
```

| Flag | Meaning                                                                      |
|---|------------------------------------------------------------------------------|
| `--filter <STR>` / `-f <STR>` | Limit verification to modules whose file name matches.                       |
| `--only <FN>` / `-o <FN>` | Verify only the named function (e.g., `mod::increment` or just `increment`). |
| `--verbosity <LEVEL>` / `-v` | Diagnostic verbosity: `error`, `warn`, `info`, `debug`.                      |
| `--proc-cores <N>` | Parallelism for verification conditions.                                     |
| `--vc-timeout <SECS>` | Per-function SMT solver timeout (soft).                                      |
| `--random-seed <N>` | Pin the prover's random seed .                                               |
| `--check-inconsistency` | Smoke-check that specs aren't contradicting themselves.                      |
| `--dump` | Dump intermediate step results (bytecode, generated SMT, Z3 trace, …) to files alongside the package. Useful when debugging a verification failure. |

## `aptos move lint`

Run the Move linter, which surfaces stylistic and correctness warnings beyond what the compiler itself reports.

```shellscript filename="Terminal"
aptos move lint
aptos move lint --checks strict
aptos move lint --checks "strict,-needless_visibility,cyclomatic_complexity"
aptos move lint --list-checks            # show every available lint, grouped by tier
```

| Flag | Meaning |
|---|---|
| `--checks <SPEC>` | Comma-separated list of tiers (`default`, `strict`, `experimental`, `all`) and individual lints. Prefix a name with `-` to exclude it. |
| `--list-checks` | Print all available lints grouped by tier, then exit. |

_See also: [Aptos Move Lint](https://aptos.dev/build/smart-contracts/linter)._

## `aptos move fmt`

Format Move source files in place using `movefmt`.

`movefmt` is a separate binary that the CLI doesn't bundle. Install (or update) it once before running `fmt`:

```shellscript filename="Terminal"
aptos update movefmt
```

Then:

```shellscript filename="Terminal"
aptos move fmt                                 # rewrite files in place
aptos move fmt --emit-mode diff                # show a diff instead
aptos move fmt --emit-mode std-out             # print to stdout
aptos move fmt --file-path src/foo.move src/bar.move
```

| Flag | Meaning |
|---|---|
| `--emit-mode <MODE>` | One of `overwrite` (default), `new-file`, `std-out`, `diff`. |
| `--package-dir <PATH>` | Format every `.move` file in the package. Mutually exclusive with `--file-path`. |
| `--file-path <PATH>...` | Format a specific list of files. |
| `--config-path <PATH>` | Use a specific `movefmt.toml` instead of searching. |
| `--config <KEY=VAL,...>` | Override individual config entries from the command line. |

_See also: [Formatting Move Contracts](https://aptos.dev/build/cli/formatting-move-contracts)._

## `aptos move document` (alias: `doc`)

Generate Markdown documentation from `///` doc-comments and spec blocks. Output lands in `<package-dir>/build/<package>/doc/`.

```shellscript filename="Terminal"
aptos move document
aptos move document --include-impl
```

| Flag | Meaning |
|---|---|
| `--include-impl` | Include function implementations in the rendered docs (otherwise they're collapsed to signatures). |
| `--landing-page-template <PATH>` | Use a custom landing-page template. |

## `aptos move clean`

Remove the `build/` directory and other derived artifacts from a package. Prompts before deletion unless `--assume-yes` is set.

```shellscript filename="Terminal"
aptos move clean
aptos move clean --assume-yes
```

## `aptos move disassemble`

Render Move bytecode as readable assembly (`.mv.masm`). Useful for inspecting downloaded packages or build output. Exactly one of `--package-dir` or `--bytecode-path` must be provided.

```shellscript filename="Terminal"
# A directory of .mv files (e.g., from `aptos move download --bytecode`)
aptos move disassemble --package-dir MyPkg/bytecode_modules

# A single .mv file
aptos move disassemble --bytecode-path build/MyPkg/bytecode_modules/my_module.mv
```

| Flag | Meaning |
|---|---|
| `--package-dir <PATH>` | Process every `.mv` in the directory. |
| `--bytecode-path <PATH>` | Process a single `.mv` file. |
| `--is-script` | Treat the input as a transaction script rather than a module. |
| `--code-coverage-path <PATH>` | Annotate the output with coverage from a `.mvcov` file. |
| `--print-metadata-only` | Print only the bytecode/metadata version, then exit (with `--bytecode-path`). |

## `aptos move decompile`

Decompile bytecode toward Move source (`.mv.move`). Same input shape as `disassemble`. The output is a best-effort approximation of the original source — useful for understanding on-chain code whose source isn't available.

```shellscript filename="Terminal"
aptos move decompile --package-dir MyPkg/bytecode_modules
aptos move decompile --bytecode-path some_module.mv
```

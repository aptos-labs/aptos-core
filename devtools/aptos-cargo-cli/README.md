# Aptos Cargo CLI test determination

## Selection model

The [selector](src/test_selection.rs) narrows affected Rust **packages** to subsystem
boundaries and selects named E2E **suites** using the `determinator` dependency
engine. Build/lint commands use workspace dependency selection. Specialized tests
outside the registered E2E runners follow their workflow gates. Individual
test-function selection is not implemented.

[`.config/test-subsystems.toml`](../../.config/test-subsystems.toml) defines the
boundaries; `--subsystem-config` selects another repository-relative file. Add
subsystems through configuration. Move is configured as follows:

```toml
[subsystems.move]
roots = ["third_party/move", "aptos-move"]
ignored_paths = [
    "third_party/move/documentation/**",
    "third_party/move/move-prover/doc/**",
]
selection = "affected"
related_test_roots = ["api"]
related_test_packages = [
    "aptos-db-indexer",
    "aptos-types",
    "aptos-executor-benchmark",
    "aptos-indexer-grpc-fullnode",
]
```

“Global” means top-level; “Subsystem” means `[subsystems.<name>]`.

| Setting | Scope | Meaning |
| --- | --- | --- |
| `version` | Global only | Schema version: `1`. |
| `subsystems` | Global only | Named subsystem definitions. |
| `global_test_inputs` | Global only | Select all eligible workspace packages and registered E2E suites. |
| `unmatched_changes` | Global only | Must be `legacy`: union legacy selection for the full change when any relevant path is unconfigured. |
| `[e2e_tests.<name>]` | Global only | Define suite dependencies through `affected_packages` and `input_paths`. |
| `roots` | Subsystem | Activate the subsystem and discover eligible Cargo packages by directory. |
| `related_test_roots`, `related_test_packages` | Subsystem | Additional eligible packages; do not activate the subsystem themselves. |
| `selection` | Subsystem | `affected` intersects dependency impact with eligible packages; `all` selects every candidate. |
| `always_test_packages` | Subsystem | Always included when the subsystem activates. |
| `path_rules` | Subsystem | Add package seeds for filesystem dependencies missing from Cargo; matching rules are additive and can activate a subsystem. |
| `e2e_tests` | Subsystem | Eligible named suites, selected when a declared dependency is affected or an input changes. |
| `ignored_paths` | Both | Global: repository-wide. Subsystem: within its roots, without suppressing overlapping subsystems. Global inputs and explicit package/E2E mappings take precedence. |

Changes are measured from the merge base of HEAD and `--base` (default
`origin/main`), including tracked staged/unstaged edits, deletions, and both rename
endpoints. Untracked files are excluded. Each path seeds its nearest owning
workspace package in the base and analyzed revision graphs, plus explicit mappings. Dependency
analysis includes normal, build, and development dependencies with resolver V2.
Paths may cross packages outside a subsystem; only eligible packages are selected.
Results are unioned, sorted, and filtered through
`TARGETED_UNIT_TEST_PACKAGES_TO_IGNORE` in [src/lib.rs](src/lib.rs).
Deleted packages are never executed; an empty selection skips Nextest.

Inputs within a subsystem but without an owner or mapping select all of its
candidates and listed E2E suites. API-only changes use legacy fallback because
API is a related test root, not a subsystem. Mixed Move/unconfigured changes also
include legacy selection for the full change. Global changes override boundaries;
only ignored changes select no targeted tests. Changes to the selected config
always force global coverage, even if its rules try to ignore that change.

Configured file mappings cover framework Move sources/manifests and cached-package
consumers, stdlib inputs read by compiler/Prover harnesses, and Aptos examples.
Extend these for dependencies absent from Cargo; there is no extension allowlist.
Roots match directory components; repository-relative globs use `*` within a
component and `**` across directories. Unknown fields, invalid paths/patterns,
and unresolved explicit names are errors.

## E2E configuration and CI

Move selects **no automatic E2E suites**. Registered runners remain available for
other subsystems, legacy fallback, global changes, and the nightly backstop.
Top-level definitions supply dependencies, for example
(abbreviated; see the configuration for all input paths):

```toml
[e2e_tests.cli-e2e]
affected_packages = ["aptos"]
input_paths = ["crates/aptos/e2e/**"]
```

E2E dependency analysis uses the full affected graph before the Rust boundary is
applied. Eligibility alone does not select a suite. List accepted names and their
runners without Git or Cargo metadata:

```bash
cargo x list-e2e-tests                  # Also supports --format json
```

The shared [registry](../../.github/actions/e2e-test-determinator/registry.json)
covers CLI/API, PR execution performance, current-node faucet tests, and Forge
E2E and compatibility. Unknown E2E names or missing referenced definitions fail both `subsystem`
and `compare` with a nonzero exit. Registry tests verify workflow jobs and required
nightly coverage. A new runner needs registration, dependencies, workflow wiring,
and nightly coverage; assigning a registered runner is configuration-only.

The [E2E action](../../.github/actions/e2e-test-determinator/action.yaml) emits
selected names; consumers gate jobs or set `SKIP_JOB`. Workflow event, label,
permission, same-repository, image-build, and documentation gates also apply.
Planner failures fail visibly, including required checks. Push/manual/nightly
runs retain full selection. `CICD:run-all-e2e-tests` bypasses selection, but
specialized label/permission gates apply. Explicit performance overrides force
that runner. The CLI runner executes its complete ordered Python suite when selected.

These suites bypass subsystem selection and cannot appear in `e2e_tests`:

| Manual PR suite | Activation |
| --- | --- |
| MonoMove parity | `mono-move-e2e-tests` label on same-repository PRs, or dispatch. |
| MonoMove performance | `mono-move-e2e-perf` label on same-repository PRs, or dispatch. |
| Forge framework upgrade | `CICD:run-framework-upgrade-test` label; separate scheduled/dispatch workflow. |
| Forge consensus-only performance | `CICD:run-consensus-only-perf-test` label. |
| Forge multiregion | `CICD:run-multiregion-test` label. |
| Production-network faucet | `CICD:non-required-tests` label. |

Nightly calls these independently. Execution performance remains registered because
its `LAND_BLOCKING` flow supports PRs/automerge; dispatch uses `CONTINUOUS`.

Flow evaluation infrastructure in `aptos-move/flow/evaluation/spec-inference`
is **manual-only** and has no E2E runner; only its publication-bundle test runs,
as part of general lints.

## Modes and inspection

| Mode | Execution |
| --- | --- |
| `legacy` (default) | Workspace dependency impact without subsystem boundaries; E2E selection follows workflow gates. |
| `compare` | Execute legacy; report subsystem differences. |
| `subsystem` | Execute subsystem selection. |

```bash
cargo x --determinator compare --base origin/main test-plan --format json
APTOS_TEST_DETERMINATOR=subsystem cargo x targeted-unit-tests
```

`--determinator` overrides the environment. Put global options before the command;
subsequent flags are forwarded to Cargo. Explicit `-p` bypasses automatic Rust
selection, subject to runner exclusions, and cannot drive the E2E action. CI uses
the repository variable `APTOS_TEST_DETERMINATOR` and the actual PR base branch.
E2E workflows build the planner from the trusted base before analyzing PR sources,
with read-only permissions.
Legacy E2E planning uses the workflow revision; compare/subsystem read the
configuration from the analyzed revision.

JSON plans include revisions, changed paths, seeds, selected packages/E2E suites,
reasons, exclusions, and legacy/subsystem differences. Dependency-path traces are
not emitted. Git and required metadata failures fail the command. Compare mode
retains legacy execution on subsystem-evaluation errors except invalid E2E names.
The targeted-test command prints the same plan it executes.

## Nightly backstop

The [nightly workflow](../../.github/workflows/nightly-full-suite.yaml) bypasses
selection and documentation skips while preserving legacy CI test eligibility
and exclusions, running the workspace baseline, registered
E2E suites, and manual PR suites listed below. Required failures or skipped
suites fail the aggregate result; independent suites continue.

| Coverage | Execution |
| --- | --- |
| Rust baseline | Workspace Nextest (`ci`, three retries), doc tests, VM feature validation, and framework bundle freshness. |
| Dedicated Rust suites | Eight smoke partitions and batch encryption with Node/pnpm. |
| Application E2E | CLI against devnet/testnet/mainnet, API specs, full execution performance, MonoMove performance/parity, faucet against the dispatched SHA/devnet/testnet, and five deployed Forge variants. |
| CI tooling | Docker release-image and Python selection/alert tests. |

Tests and images use the dispatched SHA; comparison networks use released images.
Consensus-only images have separate readiness checks and build locks. Benchmark
jobs serialize with `queue: max` (100 pending jobs). Logs, available JUnit results,
CLI output, API specs, and smoke failure artifacts are retained. Tests respect the
profile exclusions in [`.config/nextest.toml`](../../.config/nextest.toml).
Production replay, extended Forge campaigns, and special Prover runs have separate
schedules. Standalone scripts are not auto-discovered.

[PIES](https://github.com/aptos-labs/internal-ops/pull/9422) dispatches `main` daily
at **09:00 UTC (01:00 PST / 02:00 PDT)**. Land the workflow before deploying that
registration. Manual dispatch is supported. Each failed attempt sends a
consolidated alert, including failures outside Move, to `#feed-move-alerts` via
`EXECUTION_PERF_SLACK_WEBHOOK_URL`; add a notification step for any further
channel. Alerts link the revision, failed suites, logs/artifacts, and available
details. Missing dispatches or runs cancelled before
notification need external scheduler monitoring.

## Validation and rollout

Local checks cover dependency propagation, boundaries, fallback, renames/deletions,
CLI failure behavior, registry wiring, image readiness, and alert summaries:

```bash
cargo check -p aptos-cargo-cli
cargo test -p aptos-cargo-cli
cargo clippy -p aptos-cargo-cli --all-targets -- -D warnings
cargo +nightly fmt -p aptos-cargo-cli --check
python3 -m unittest discover -s .github/actions/e2e-test-determinator -p '*_test.py'
python3 -m unittest discover -s .github/actions/nightly-test-summary -p '*_test.py'
pnpm test docker/__tests__ --runInBand
```

Actionlint 1.7.12 needs its unsupported `queue` diagnostic excluded
for GitHub's `queue: max` setting.

1. Review dry-run selections; the appendix records four completed cases. Extend
   checks to framework/fixture, API-only, Python-only, and mixed-source changes.
2. Land with `legacy`; enable `compare` on PRs containing the selector and review
   omitted packages and missing file dependencies.
3. Manually run the full suite after landing:
   `gh workflow run nightly-full-suite.yaml --repo aptos-labs/aptos-core --ref main`.
   Verify every required suite, tested SHA, artifacts, and aggregate result.
4. On a temporary validation branch, replace expensive jobs with lightweight
   passing/failing jobs to check aggregation and Slack delivery, including a
   failure outside Move. Do not merge those substitutions; green runs send no alert.
5. Deploy PIES, trigger Cloud Scheduler's **Run now**, and verify daily dispatch.
   Enable `subsystem` after these checks; reset to `legacy` or unset the variable
   to roll back. Passing local tests do not establish live CI or alert delivery.

## Appendix: selection accuracy

Four source-change probes use the configuration above, excluding manual suites
from selection. An isolated worktree changed one tracked file at a time,
with all-features base metadata cached and `--base HEAD` excluding implementation
changes. An empty-diff control selected zero Rust packages.

| Changed component | Rust: legacy | Rust: subsystem | E2E: subsystem |
| --- | ---: | ---: | ---: |
| [Bytecode verifier](../../third_party/move/move-bytecode-verifier/src/lib.rs) | 198 | 104 | 0 |
| [Move compiler](../../third_party/move/move-compiler-v2/src/lib.rs) | 140 | 74 | 0 |
| [MonoVM runtime](../../third_party/move/mono-move/runtime/src/lib.rs) | 114 | 48 | 0 |
| [Storage, outside Move](../../storage/aptosdb/src/lib.rs) | 53 | 53 | 6 |

Each Move case selects exactly the legacy-affected Rust packages within
`third_party/move`, `aptos-move`, `api`, and the four related packages above,
with no automatic E2E suites.
Storage matches legacy exactly, confirming fallback. These are selection checks,
not executed test results or proof of every
filesystem dependency.

To reproduce in an isolated checkout at the baseline, cache metadata before edits:

```bash
mkdir -p target/aptos-x-tool
cargo metadata --locked --all-features --format-version 1 > "target/aptos-x-tool/metadata-$(git rev-parse HEAD).json"
```

Append a newline to one linked source file, run `cargo x --base HEAD --determinator
subsystem test-plan --format json`, then repeat with `--determinator compare`.
Restore the file before the next case. The JSON plans contain the full package lists.

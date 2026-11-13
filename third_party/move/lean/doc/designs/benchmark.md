# Verification benchmark

A nightly benchmark measures how long Verex takes to verify a fixed set of
standard problems and plots each problem's cost over the recent nightly
runs. It runs on `main`, with a report published on GitHub Pages; pull
request runs are opt-in. It is a monitor, not a gate: it never fails a pull
request. The `DenotePerformance` and `CompositionPerformance` baselines in
`verex-ir` ([`performance.md`](performance.md), "How to measure") remain the
regression gates. The same commands run locally and compare a local run with
the nightly history. The problem set and the open decisions are in
[`verification-benchmarks.md`](../status/verification-benchmarks.md).

## Goals

- 20–30 standard problems, small to heavy (`ordered_map`), read from the
  tree at the benchmarked commit, never copied.
- Cost in wall time, split into elaboration, verification, and overall;
  heartbeats beside it as the machine-independent measure of the
  elaborator's work.
- Curves over a window of recent runs, per problem and for the whole suite.
- History kept as the results of recent CI runs; no separate store.
- A local run compared with the CI runs on GitHub.

Not goals, for now: failing on a regression, benchmarking pull requests by
default, comparing with the Boogie backend.

## Problems

A problem is one verification input in the tree: Move modules of an
in-tree package, a VerexSL file, or a Rust file with its specification.
The manifest `bench/problems.toml` (under `third_party/move/lean`) names
each problem and where its input lives, relative to the repository root:

```toml
[[problem]]
name = "ordered_map"
kind = "move"
package = "aptos-move/framework/aptos-framework"
modules = ["aptos_framework::ordered_map"]

[[problem]]
name = "quicksort"
kind = "lean"
file = "third_party/move/lean/verex-e2e-tests/VerexE2ETests/Check/Examples/Quicksort.lean"

[[problem]]
name = "bounds"
kind = "rust"
file = "third_party/move/lean/verex-e2e-tests/VerexE2ETests/SourceVerify/bounds.rs"
spec = "third_party/move/lean/verex-e2e-tests/VerexE2ETests/SourceVerify/bounds.spec.lean"
repeat = 3
```

A Move problem with `dev = true` compiles its package in dev mode, with
its `[dev-addresses]`, as the higher-order paper's examples need.

`bench/rust` holds Rust counterparts of Move problems, named `rust_<name>`, so
coverage and cost are tracked on the same algorithms in both languages:
`math64`, `bit_vector`, `simple_map`, and `lower_bound`, whose Move counterpart
is `bench/package/sources/lower_bound.move`, since `ordered_map` does not verify
its search. Their false twins are `SourceVerify/bench_<name>_errors.rs`. A proof
of a Rust problem stands in its `.spec.lean`.

A problem may declare `expected_rejections`, an array of fully qualified
verification target names. AMM declares its intentionally non-compliant fee
constructor this way. The benchmark reports **expected rejection** only when
exactly those targets are rejected, every other measured target verifies,
and their diagnostic counts account for all errors. An accepted negative is
an **unexpected acceptance**; a timeout, missing target, or unrelated error
remains a problem. This label describes the verifier's outcome, rather than
claiming that a satisfiability solver produced `sat`.

Suite totals include all available measurements, including rejected targets
and work spent before a timeout. Subset runs have no suite total. Comparisons
include common measured problems and display both statuses; a previously
rejected problem becoming verified therefore does not add its entire cost to
the suite for the first time. Historical measurements without heartbeat data
contribute no heartbeat count, so their totals represent available measurements.

A problem's input is whatever the tree holds at the commit being measured,
so a curve can move because the input changed rather than the verifier.
Each result records the git object id of its input (`git rev-parse
HEAD:<path>` of the package directory or file), and the report marks the
runs where it changed. The copies under `verex-e2e-tests/.../MoveToVerexSL`
are not used; they have already drifted from the tree (`cmp`, `features`).

## Measurements

A problem runs in its own process, natively (the `verex-move` executable
runs the elaborator natively, while `lake env lean` interprets it;
[`performance.md`](performance.md), "How to measure"). The measures, main one first:

- **Wall time per phase.** The phase clock of `VerexSL.Perf` charges
  wall time to the innermost running phase (load, frontend, render,
  lowering, certification, verification). The report groups the phases as:
  - *elaboration*: lowering and certification: the modules lowered to
    linked units, and the definitions and certificates the proofs use;
  - *verification*: the proofs;
  - *overall*: the whole run, including loading, the frontend, parsing,
    and message mapping.
- **CPU time** of the problem's process, which the driver reads when the
  process exits (`os.wait4`). It counts every thread of the process, the
  Lean kernel's checks included, and is less sensitive than wall time to
  other load on the machine; it exists only for the whole run.
- **Heartbeats per phase**, read with `IO.getNumHeartbeats` at the same
  boundaries as wall time, and grouped the same way. Heartbeats count per
  thread, and Lean elaborates commands on threads of its own, so the clock
  charges an interval its heartbeats only when it begins and ends on one
  thread (`IO.getTID`), and a run's overall heartbeats are the sum of its
  phases': heartbeats outside every phase, such as parsing, are not
  counted.
- **Per target**, the wall time and heartbeats of each verified function,
  from `Perf.measure` (`Perf.measuring` on, the object count off: it is
  the gate's measure and would add its own traversal to verification).
- **Outcome**: the status (`verified`, `failed`, `timeout`, `crashed`),
  the number of errors, and the first ten error messages.

Wall time is the main measure because it is the only one that covers all
of the cost. Heartbeats count the elaborator's allocations on the
elaborating thread. They miss:

- kernel type checking, which is most of certification's cost;
- work on other threads, such as asynchronous kernel checks;
- the external Move compile in the frontend;
- native code that does not allocate.

They are the elaborator and solver part of the cost only. Much of
verification's cost is in kernel-checked certificates
([`performance.md`](performance.md)), which heartbeats do not see.

Wall time depends on the machine and on other load on it. The benchmark
therefore runs on a dedicated runner type with nothing else running
(below), runs problems one after another with `LEAN_NUM_THREADS` fixed and
recorded, and repeats small problems: a problem with `repeat = 3` in the
manifest (those under about 30 s) runs three times, and its result is the
median, with the spread recorded. Heavy problems run once; their noise is
small relative to their cost. The noise on the runner is not yet measured
([`verification-benchmarks.md`](../status/verification-benchmarks.md), "Open
decisions").

Heartbeats serve two purposes beside wall time:

- **Attribution.** The same input, toolchain, and build spend the same
  heartbeats on any machine; repeats of a problem agree to within 0.001 %.
  A wall-time change with flat heartbeats is in the kernel, in other
  threads, or noise; a change in both is more work in the elaborator or
  the solver.
- **Comparison across machines.** Heartbeats compare a local run with CI
  directly; wall time does not.

A failing target spends its whole heartbeat budget, and the time that
takes, before it fails, so its problem's cost reflects the budget rather
than the proof. Results mark failed targets, and the suite totals are
summed over the problems that verify completely, with the others counted
beside them.

Runs use the verifier's default options. The profiler is never on: its
own counting inflates heartbeats and adds time. Before the first problem,
the driver loads the environment once untimed (`verex-bench warmup`), so
the first problem does not pay for a cold file cache.

## Components

### Module selection

A problem names its modules, and only what verifying them reads is
rendered and lowered, not the whole package and its dependencies. The
exchange exporter
(`move-model-exchange`, `select.rs`) selects modules by qualified name
(`module`, `address::module`, `alias::module`) and computes what verifying
them reads: the modules their code calls or mentions in a type, the modules
their specifications use, transitively, and the modules whose global
invariants read memory declared in one of those. A model built without
bytecode, as the AST export builds it, records only the specifications' part
of a module's uses, so the code is walked for the rest. The selection
serves:

- `move exchange --format ast --modules <a::m>,…`, which exports the
  closure, dependencies included;
- `verex-move verify <package> --modules <a::m>,…`, which verifies the
  named modules and links the rest of the closure (without `--modules`, it
  applies `--filter` to a package directory it exports itself);
- `prove --lean --filter <part>`, which exports only the closure of the
  package modules whose source file name contains the filter;
- the benchmark's Move problems.

### `verex-bench`

The executable `verex-bench` in `verex-e2e-tests`, which links the Move,
Rust, and VerexSL frontends and the Check fixtures' support, runs one
problem and writes its result as JSON:

```bash
verex-bench move <package> --modules <a::m>,… [--dev] --out result.json
verex-bench lean <file.lean> --out result.json
verex-bench rust <file.rs> [--spec <file.spec.lean>] --out result.json
verex-bench warmup
```

A `lean` problem is elaborated with the imports of its header, natively.
Every run turns on per-target measurement with the object count off. A
Rust problem runs from `third_party/move/lean`, where the Rust exporter is
found. A result, to which the driver adds the problem's name, kind, group
(the package name of a Move problem, `VerexSL` or `Rust` otherwise), input
id, CPU time, and repeats:

```json
{
  "status": "verified",
  "wall_ms": { "load": 385, "frontend": 102, "render": 6, "lowering": 1,
               "certification": 74, "verification": 96, "total": 819 },
  "heartbeats": { "load": 4747306, "frontend": 14381, "render": 254280,
                  "lowering": 46851, "certification": 1485977,
                  "verification": 2548601, "total": 9097396 },
  "targets": [ { "target": "«0x1».error::canonical", "wall_ms": 96,
                 "heartbeats": 2548593 } ],
  "errors": 0,
  "error_messages": []
}
```

### `scripts/verex-bench.py`

The driver, written in Python like the other performance jobs, uses only
the standard library and the `gh` CLI:

- `run [--only <names>] [--out results.json] [--package <dir>] [--jobs N] [--threads N] [--timeout SECONDS]`
  reads the manifest, builds and warms up once, then runs independent problems
  in separate process groups. `--jobs` limits simultaneous problem processes:
  by default, at most four locally (bounded by CPU count) and one in CI.
  `--jobs 1` runs the problems sequentially. Repeats of one problem always run
  sequentially in the same worker, and final results retain manifest order.
  Launch progress shows each process's PID and the active process count;
  completion progress appears as problems finish.

  Each complete problem, including all repeats, has a wall-time cap of
  **300 seconds (five minutes)** by default. A manifest `timeout` replaces
  that default; an explicit `--timeout` overrides both. Expiration kills the
  entire process group and records a timeout, then other problems continue.
  An earlier successful repeat cannot hide a later timeout or crash. Ctrl-C,
  SIGTERM, or a runner exception stops all active groups and cancels queued
  problems; setup processes also clean up their children on interruption.
  The driver writes the run: a
  schema version, the commit and whether the tree was dirty, the date, the
  Lean toolchain, the runner, the CPU model, the problem-process limit, the
  Lean thread count per process (8 by default, or `LEAN_NUM_THREADS`), the
  branch, whether it measured a subset (`--only`), and per
  problem the median result and its repeats. A problem
  that times out or crashes is recorded with that status, and the run
  continues. Messages name files relative to the repository root, as the
  manifest does, so runs on different checkouts read alike. The executable
  runs directly, in the environment `lake env` gives it; the Move CLI is
  `APTOS_MOVE_CLI` or the checkout's `target/ci/move`. A local run is
  recorded in the local history (below); in CI, `--out` names the results.
  Compare wall times with the same `--jobs` and `--threads` settings;
  overlapping problems share machine resources. Heartbeats remain the primary
  comparison measure. Older records without `jobs` describe one problem process.
- `history [--window N] [--branch main]` fetches the results of the last
  `N` CI runs that produced results (below) into a local cache keyed by run
  id; a run's results never change. `history --local` lists the recorded
  local runs of the current branch instead.
- `report [--current results.json] [--local results.json] [--local-runs] [--full-runs-only]
  [--window N] [--html <file>] [--markdown <file>] [--slack <file>]`
  renders the history, with the running CI run's results (`--current`), a
  local run, or the branch's recorded local runs appended, as the outputs
  described below; without an output it prints the markdown.
- `compare [before] [after]` compares two runs of one machine by wall time
  and heartbeats alike, per problem and over the problems both verified.
  A run is a results file or `@N`, the N-th recorded local run of the
  current branch counting back from `@-1`, the latest; by default the last
  two, `@-2` and `@-1`.

## History

Every CI run uploads its `results.json` as the workflow artifact
`verex-bench-results`, retained for 90 days. The history is the results
of the most recent runs:

```bash
gh run list --workflow verex-bench.yaml --branch main --status completed \
  --limit 3N --json databaseId,headSha,createdAt,url
gh run download <databaseId> --name verex-bench-results
```

The window defaults to 30 runs, a month of nightlies; retention bounds it
at about 90. A run counts by its results artifact, not by its conclusion:
the benchmark fails only when it could not produce results (a build
failure, a missing manifest entry), never because a problem fails to
verify, and a run whose report or publication failed afterwards still
measured, so its point is kept.

Each point on a curve is annotated with what changed since the previous
point: the problem's input, the Lean toolchain, or its status. A problem
added to the manifest starts its curve at the run that added it, and a
removed problem stops. The HTML index and problem sections use the latest
full run's problem list, plus any subsequent subset measurements. Removed
problems therefore disappear from the current page while their measurements
remain in historical artifacts and historical suite totals.

## Reports

- **Curves.** `report --html` writes one self-contained page with an overall
  heartbeat chart for the suite, and elaboration, verification, and overall
  heartbeat series for each problem. It includes markers for the annotations
  above, history tables with the same heartbeat phases, and a table of each
  problem's most expensive targets in the latest run. Wall-time charts and
  history tables, in seconds, follow the heartbeat results in sections
  collapsed by default, with the spread of repeated problems shown as a band.
  The global and per-problem tables of most expensive targets are also
  collapsed by default. The problems are listed by group, with an index that
  marks those not verified in the latest run. Heartbeats are in the largest
  of k, M, and G of which a chart's largest value is at least one.
  The charts are inline SVG generated by the script, so
  the page needs no network access and no plotting library. CI publishes the
  latest main report at the stable repository Pages URL,
  `https://aptos-labs.github.io/aptos-core/verex-bench/`. Each pull request has
  its own page at `verex-bench/<PR-number>/`, updated by its benchmark runs.
  Publishing main or another PR preserves that page. Individual runs also
  retain their HTML as workflow artifacts.
- **Step summary.** `report --markdown` writes the job summary, headed by
  a link to the main or PR report page: one row per
  problem with its status, the three wall times, their change against the
  previous run and against the median of the window, the change in overall
  heartbeats, and a sparkline (`▁▂▃▄▅▆▇█`) of overall wall time.
- **Slack payload.** `report --slack` can write a message for manual use, but
  the benchmark workflow does not send it. The payload contains the commit;
  the suite's overall wall time with the same two changes;
  the number of problems verified, failed, and timed out; the problems whose
  overall wall time moved against the window median by more than the noise
  threshold (10 %, until the runner's noise is measured), whose heartbeats moved
  by at least 5 %, or whose status or input changed, each with its
  sparkline; and links to the chart page and workflow run when provided.

## CI

The workflow `.github/workflows/verex-bench.yaml`:

- **Triggers.** `workflow_dispatch` for the nightly. Repository policy
  prohibits GitHub cron: PIES dispatches the nightly on `main`, registered
  in `internal-ops` as the nightly full suite is
  (`devtools/aptos-cargo-cli/README.md`). Manual dispatch takes
  `problems` (a subset) and `window`; GitHub dispatches only a workflow
  that is on the default branch. A pull request of this repository (not a
  fork) runs the benchmark on its head when it carries the
  `CICD:run-verex-bench` label, again on every push while it does, and
  its report compares it with `main`'s nightlies.
- **Job `bench`.** The dedicated benchmark runner of the execution
  performance test (`benchmark-c3d-60`), in its concurrency group
  (`execution-benchmark-benchmark-c3d-60`, queued, never cancelled), so that
  no other benchmark shares the machine. Steps: check out the commit; set
  up Rust (`.github/actions/rust-setup`) and Python 3.12; install Lean
  (`scripts/dev_setup.sh -b -p -l -k`); build the ci-profile `move` CLI;
  install the Rust exporter's pinned toolchain; `lake build verex-bench`
  in `verex-e2e-tests`; `scripts/verex-bench.py run`; upload
  `verex-bench-results`. `timeout-minutes: 150`.
- **Job `report`.** Needs `bench`, on a small runner, with `permissions:
  actions: read` and `GH_TOKEN: ${{ github.token }}` for the history. Runs
  `verex-bench.py report --current` over the downloaded results of this
  run, which the history lists only once the run has completed; appends
  the markdown to `$GITHUB_STEP_SUMMARY`; and uploads the page as
  `verex-bench-report`.
- **Job `publish`.** Needs `report`, with `permissions: contents: write`,
  runs for dispatches on `main` and PRs from the same repository. It is the
  only job that writes and runs no code of the benchmarked commit.
  Commits the page to the `gh-pages` branch (a sparse checkout of
  `verex-bench/`, rebasing and retrying when a concurrent deploy moved
  the branch), replacing only `verex-bench/index.html` for main or
  `verex-bench/<PR-number>/index.html` for a PR. Because `GITHUB_TOKEN` pushes
  do not trigger branch-based Pages builds, it uses `pages: write` to request
  a build and waits for a successful build containing the published commit.
  It then uses `pull-requests: write` to append a PR comment linking to the
  charts and the workflow run. Failed publication does not post a completion
  comment.
- **Concurrency.** One group per ref, runs never cancelled, so that every
  nightly produces a point.

## Local use

```bash
cargo build --locked --profile ci -p aptos-move-cli --features binary --bin move
python3 third_party/move/lean/scripts/verex-bench.py run [--only math128,ordered_map]
```

A local run is recorded in the local history, and the files of local runs
stay in `third_party/move/lean`, ignored by git:

- `local_benchmark_history.jsonl`: the latest 100 local runs, each with its
  branch;
- `local_benchmark.json`: the latest local measurement;
- `local_benchmark.html`: the latest local run against the main CI history,
  regenerated by the script after every run; use `report --local-runs`
  explicitly to include intermediate local runs. The page starts with a
  collapsed section of the 20 most expensive targets in the latest run,
  ranked by heartbeats, with outcome status and links to their problem sections;
- `local_benchmark_logs/`: the problems' logs of the latest local run.

A series of changes on a branch, one command after each (commit first):

```bash
python3 third_party/move/lean/scripts/verex-bench.py run --only math128,ordered_map
```

`run` builds `verex-bench` from the current sources, measures, records the
run, rewrites the page, and prints its comparison with main.
`compare` only reads recorded runs: `history --local` lists the
series (`@-N … @-1`), and `compare @-4 @-1` gives the change since `@-4`.
Keep the same `--only` set through a series and commit each change, since a
run is labeled by its commit (`+` when the tree was dirty). Heartbeats tell
whether the elaborator's work changed, wall time whether the change shows;
an agent follows its own optimizations this way. The suite total compares
full runs only.

`run --package <dir>` builds and runs `verex-bench` of another copy of
`verex-e2e-tests`, such as one on a native disk.

To select process parallelism explicitly, or run one problem with one Lean
worker at a time:

```bash
python3 third_party/move/lean/scripts/verex-bench.py run --jobs 4 --threads 8 --timeout 300
python3 third_party/move/lean/scripts/verex-bench.py run --jobs 1 --threads 1 --timeout 300
```

These are separate controls: `--jobs` launches independent benchmark processes;
`--threads` sets `LEAN_NUM_THREADS` inside each process. `--threads 1` also selects
`verex.jobs=1` and `Elab.async=false`, so each command executes its proofs directly
instead of waiting for asynchronous tasks on its only worker. These scheduling
settings override conflicting `VEREX_OPTIONS` entries while preserving other
options. Both source inputs and Lean files consume `VEREX_OPTIONS`.

`--jobs 1 --threads 1` therefore runs one benchmark process, one Lean worker,
and one proof target at a time. Multiple threads retain the existing parallel
target scheduler; `VEREX_OPTIONS=verex.jobs=1` can independently serialize its
targets. No heartbeat budget is changed by these runner controls.

Synchronous verification includes its kernel checks in the target heartbeat
measurements; the parallel path checks declarations asynchronously. Compare
heartbeat results with the same scheduling mode. The existing performance
baselines describe the parallel path and are unchanged.

To leave partial diagnostic runs out of the HTML:

```bash
python3 third_party/move/lean/scripts/verex-bench.py report --local-runs --full-runs-only
```

This filters local records marked `subset: true` from the report. Full runs
remain visible even if individual problems failed or timed out. The raw local
history is unchanged and remains available to `history --local` and `compare`.

The page fetches the CI history with the user's `gh` login. Its comparison
table measures the latest local run against the latest CI run at or before
the merge base of the local branch with `main`, so that changes that
landed on `main` since do not show up as the local change. The HTML comparison
leads with heartbeat counts, their percentage change and verification outcomes.
Heartbeats compare directly when the toolchain matches (the report warns when
it does not). Each problem's share of the suite's wall time follows in a
section collapsed by default, since wall time differs with the machine.
The command-line report also shows these shares.
To measure a change in wall time, `compare` two local runs, before and
after, on the same idle machine.

### A local framework sample

From the Lean directory, run:

```sh
python3 scripts/verex-bench.py framework
```

This writes `local_fx_bechmark.html`, with separate results in
`local_fx_benchmark.json`, history in `local_fx_benchmark_history.jsonl`,
and per-module logs under `local_fx_benchmark_logs/`. These local artifacts
are git-ignored. The filename of the HTML page can be changed with `--html`.

The first run selects the most expensive 30% of framework modules with
specifications, ranked by prior module wall times recorded by
`verify-modules.py`. It stores that selection in `local_fx_selection.json`;
later runs reuse it. `--refresh-selection --fraction 0.3` reranks the sample.
Without prior measurements, run `verify-modules.py` on the framework first.
Only runs of the same branch and identical selection share a chart.

Each selected module is verified whole, at `--heartbeats 300`, with one
target worker per process and eight module processes by default (`--jobs`).
`--timeout` bounds each module's wall time (300 seconds by default).
`--no-build` uses the existing verifier. Do not build or change verifier
sources during a measurement.

The page reports observed verified, budget-stopped, failed, and interrupted
targets, including all measured attempts in its heartbeat totals. It labels
the selection as a sample; modules that crash or time out may have unstarted
targets, so the observed-target count is not whole-framework coverage.
Namespace preparation and whole-run elapsed time are reported separately
from the sum of module times. These results do not mix with nightly benchmark
history.

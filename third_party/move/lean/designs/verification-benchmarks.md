# Verification benchmarks

Status: 2026-09-30, implemented and run locally (stages 1–3 below); the
workflow is written but has not run in CI yet, and the PIES registration is
open.

A nightly benchmark measures how long Leaner takes to verify a fixed set of
standard problems and plots each problem's cost over the recent nightly
runs. It is a monitor, not a gate: it never fails a pull request.
The `DenotePerformance` and `CompositionPerformance` baselines in
`leaner-ir` (`perf-notes.md`, "How to measure") remain the regression gates.
The job runs in CI like the execution performance test and reports to the
same Slack channel. The same commands run locally and compare a local run
with the nightly history.

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
in-tree package, a LeanerLang file, or a Rust file with its specification.
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
file = "third_party/move/lean/leaner-e2e-tests/LeanerE2ETests/Check/Examples/Quicksort.lean"

[[problem]]
name = "bounds"
kind = "rust"
file = "third_party/move/lean/leaner-e2e-tests/LeanerE2ETests/SourceVerify/bounds.rs"
spec = "third_party/move/lean/leaner-e2e-tests/LeanerE2ETests/SourceVerify/bounds.spec.lean"
repeat = 3
```

A problem's input is whatever the tree holds at the commit being measured,
so a curve can move because the input changed rather than the verifier.
Each result records the git object id of its input (`git rev-parse
HEAD:<path>` of the package directory or file), and the report marks the
runs where it changed. The copies under `leaner-e2e-tests/.../MoveToLeanerLang`
are not used; they have already drifted from the tree (`cmp`, `features`).

The problems are chosen to cover the verifier's cost centers: arithmetic,
loops, vectors, generics, storage, intrinsic maps, cross-module calls,
crypto natives, and the kernel-heavy unit preparation. The initial set:

| Problem | Input | Covers |
|---|---|---|
| `option`, `vector`, `string`, `error` | `move-stdlib` | small modules, vector reasoning, fixed per-problem cost |
| `bit_vector` | `move-stdlib` | loops (`shift_left` about 284M heartbeats) |
| `fixed_point32` | `move-stdlib` | non-linear arithmetic |
| `features` | `move-stdlib` | bit flags over vectors (`apply_diff` about 714M) |
| `acl`, `cmp` | `move-stdlib` | vector membership, native model of `compare` |
| `math64`, `math128` | `aptos-stdlib` | arithmetic, `sqrt` (about 53 s) |
| `math_fixed`, `fixed_point64` | `aptos-stdlib` | fixed-point arithmetic |
| `simple_map` | `aptos-stdlib` | intrinsic map, insertion order |
| `comparator`, `type_info`, `capability` | `aptos-stdlib` | generics, reflection natives |
| `pool_u64` | `aptos-stdlib` | quantified map invariants (partly verified today) |
| `ristretto255` | `aptos-stdlib` | crypto natives, frame certificates (about 95 s) |
| `ed25519` | `aptos-stdlib` | crypto natives |
| `ordered_map` | `aptos-framework` | intrinsic map, sorted, iterators (about 80–110 s) |
| `Quicksort`, `OrderedMap` | `Check/Examples` | LeanerLang: nested loops, authored proofs |
| `VectorOperations`, `GenericStorage` | `Check/Vectors`, `Check/Generics` | shared terms, generic storage |
| `bounds` | `SourceVerify` | the Rust frontend |

These are 26 problems. A problem that does not verify completely today
stays in the set: its curve then also shows the progress on it. Problems
that need table intrinsics (`smart_table`, `big_ordered_map`) join once
those intrinsics are supported.

One local run on 2026-09-30 (aarch64, 8 threads, 535 s of problem time,
about 9 minutes with repeats; seconds and millions of heartbeats; `option`
ran first, before the warm-up below existed, and paid for a cold file
cache):

| Problem | Status | Elaboration s | Verification s | Overall s | Heartbeats M |
|---|---|---:|---:|---:|---:|
| `option` | verified | 7.7 | 4.5 | 13.1 | 192 |
| `vector` | verified | 1.9 | 0.1 | 3.0 | 33 |
| `string` | verified | 0.1 | 0.0 | 0.8 | 13 |
| `error` | verified | 0.1 | 0.1 | 0.7 | 9 |
| `bit_vector` | verified | 1.8 | 11.3 | 13.8 | 373 |
| `fixed_point32` | verified | 0.8 | 3.4 | 4.9 | 131 |
| `features` | verified | 12.2 | 59.7 | 73.7 | 1,447 |
| `acl` | verified | 0.5 | 3.3 | 4.6 | 133 |
| `cmp` | verified | 1.1 | 0.4 | 2.0 | 43 |
| `math64` | verified | 0.5 | 0.6 | 1.8 | 38 |
| `math128` | verified | 1.5 | 1.7 | 4.0 | 94 |
| `math_fixed` | verified | 1.9 | 1.0 | 3.7 | 74 |
| `fixed_point64` | verified | 1.2 | 4.4 | 6.4 | 181 |
| `simple_map` | verified | 0.1 | 0.0 | 0.9 | 17 |
| `comparator` | verified | 2.9 | 3.0 | 6.8 | 163 |
| `type_info` | 1 error | 9.8 | 0.3 | 11.1 | 138 |
| `capability` | 20 errors | 3.4 | 8.4 | 12.8 | 330 |
| `pool_u64` | 14 errors | 14.5 | 81.3 | 97.0 | 3,039 |
| `ristretto255` | verified | 19.1 | 5.4 | 26.0 | 532 |
| `ed25519` | verified | 12.9 | 2.2 | 16.2 | 267 |
| `ordered_map` | 19 errors | 27.8 | 159.3 | 197.1 | 4,451 |
| `Quicksort` | verified | 0.7 | 6.7 | 8.2 | 229 |
| `OrderedMap` | verified | 3.9 | 8.2 | 13.0 | 337 |
| `VectorOperations` | verified | 4.5 | 5.1 | 10.4 | 243 |
| `GenericStorage` | verified | 0.5 | 1.5 | 2.5 | 63 |
| `bounds` | verified | 0.1 | 0.4 | 1.0 | 19 |

A second run of the same commit, with other work on the machine, spent the
same heartbeats on every problem and differed in wall time by up to a
factor of two on the small problems, which is what the dedicated runner and
the repeats are for.

The failures are the verifier's, not the benchmark's: `type_info` calls a
function of its namespace at its own type parameters outside any call,
`capability` leaves residual obligations, `pool_u64` is the open intrinsic
map work (`intrinsic-maps.md`), and the in-tree `ordered_map` carries
symbolic iterator tests beyond the extract `perf-notes.md` measured.

## Measurements

A problem runs in its own process, natively (the `leaner-move` executable
runs the elaborator natively, while `lake env lean` interprets it;
`perf-notes.md`, "Reading a profile"). The measures, main one first:

- **Wall time per phase.** The phase clock of `LeanerLang.Perf` charges
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

They are the elaborator and closer part of the cost only. The cost of the
last rounds was largely in kernel-checked certificates (`perf-notes.md`),
which heartbeats do not see.

Wall time depends on the machine and on other load on it. The benchmark
therefore runs on a dedicated runner type with nothing else running
(below), runs problems one after another with `LEAN_NUM_THREADS` fixed and
recorded, and repeats small problems: a problem with `repeat = 3` in the
manifest (those under about 30 s) runs three times, and its result is the
median, with the spread recorded. Heavy problems run once; their noise is
small relative to their cost. Stage 4 measures the noise on the runner by
repeated runs of one commit, and sets the repeat counts and the report's
thresholds from it.

Heartbeats serve two purposes beside wall time:

- **Attribution.** The same input, toolchain, and build spend the same
  heartbeats on any machine; repeats of a problem agree to within 0.001 %.
  A wall-time change with flat heartbeats is in the kernel, in other
  threads, or noise; a change in both is more work in the elaborator or
  the closer.
- **Comparison across machines.** Heartbeats compare a local run with CI
  directly; wall time does not.

A failing target spends its whole heartbeat budget, and the time that
takes, before it fails, so its problem's cost reflects the budget rather
than the proof. Results mark failed targets, and the suite totals are
summed over the problems that verify completely, with the others counted
beside them.

Runs use the verifier's default options. The profiler is never on: its
own counting inflates heartbeats and adds time. Before the first problem,
the driver loads the environment once untimed (`leaner-bench warmup`), so
the first problem does not pay for a cold file cache.

## Components

### Module selection

The Move frontend verified modules by file-name filter and still rendered
and lowered every module of the package and of its dependencies; for
`ordered_map`, all of `aptos-framework`. The exchange exporter
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
- `leaner-move verify <package> --modules <a::m>,…`, which verifies the
  named modules and links the rest of the closure (and, without
  `--modules`, now applies `--filter` to a package directory it exports
  itself, which it ignored before);
- `prove --lean --filter <part>`, which exports only the closure of the
  package modules whose source file name contains the filter;
- the benchmark's Move problems.

`ordered_map` exports its closure in 1.9 s and verifies in 197 s.

### `leaner-bench`

The executable `leaner-bench` in `leaner-e2e-tests`, which links the Move,
Rust, and LeanerLang frontends and the Check fixtures' support, runs one
problem and writes its result as JSON:

```bash
leaner-bench move <package> --modules <a::m>,… --out result.json
leaner-bench lean <file.lean> --out result.json
leaner-bench rust <file.rs> [--spec <file.spec.lean>] --out result.json
leaner-bench warmup
```

A `lean` problem is elaborated with the imports of its header, natively.
Every run turns on per-target measurement with the object count off. A
Rust problem runs from `third_party/move/lean`, where the Rust exporter is
found. A result, to which the driver adds the problem's name, kind, input
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

### `scripts/leaner-bench.py`

The driver, written in Python like the other performance jobs, uses only
the standard library and the `gh` CLI:

- `run [--only <names>] --out results.json [--package <dir>] [--threads N]`
  reads the manifest, runs the warm-up, then every problem under a timeout
  (20 minutes by default, killing its process group, also when the driver
  itself is stopped) as often as its `repeat` asks, and writes the run: a
  schema version, the commit and whether the tree was dirty, the date, the
  Lean toolchain, the runner, the CPU model, the thread count (8 by
  default), and per problem the median result and its repeats. A problem
  that times out or crashes is recorded with that status, and the run
  continues. Messages name files relative to the repository root, as the
  manifest does, so runs on different checkouts read alike. The executable runs directly, in the environment `lake env`
  gives it; the Move CLI is `APTOS_MOVE_CLI` or the checkout's
  `target/ci/move`.
- `history [--window N] [--branch main]` fetches the results of the last
  `N` CI runs that produced results (below) into a local cache keyed by run
  id; a run's results never change.
- `report [--current results.json] [--local results.json] [--window N]
  [--html <file>] [--markdown <file>] [--slack <file>]` renders the
  history, with the running CI run's results (`--current`) or a local run
  appended, as the outputs described below; without an output it prints
  the markdown.
- `compare <before.json> <after.json>` compares two runs of one machine,
  such as a local run before and after a change, by wall time and
  heartbeats alike.

## History

Every CI run uploads its `results.json` as the workflow artifact
`leaner-bench-results`, retained for 90 days. The history is the results
of the most recent runs:

```bash
gh run list --workflow leaner-bench.yaml --branch main --status completed \
  --limit 3N --json databaseId,headSha,createdAt,url
gh run download <databaseId> --name leaner-bench-results
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
removed problem stops.

## Reports

- **Curves.** `report --html` writes one self-contained page: for the suite
  and for each problem, a chart of elaboration, verification, and overall
  wall time over the window, with the spread of repeated problems as a
  band; a second chart of the same three in heartbeats; markers for the
  annotations above; and a table of the problem's most expensive targets
  in the latest run. The charts are inline SVG generated by the script, so
  the page needs no network access and no plotting library. CI publishes it
  on the repository's Pages site: every run at
  `https://aptos-labs.github.io/aptos-core/leaner-bench/runs/<run id>/`,
  pull request runs included, and the latest nightly at
  `https://aptos-labs.github.io/aptos-core/leaner-bench/`; the pages of the
  90 most recent runs are kept.
- **Step summary.** `report --markdown` writes the job summary, headed by
  the link to the run's page: one row per
  problem with its status, the three wall times, their change against the
  previous run and against the median of the window, the change in overall
  heartbeats, and a sparkline (`▁▂▃▄▅▆▇█`) of overall wall time.
- **Slack.** `report --slack` writes the message posted after each nightly
  run: the commit; the suite's overall wall time with the same two changes;
  the number of problems verified, failed, and timed out; the problems whose
  overall wall time moved against the window median by more than the noise
  threshold (10 % until stage 4 measures the noise), whose heartbeats moved
  by at least 5 %, or whose status or input changed, each with its
  sparkline; and links to the run's page and the run. Being a monitor, it
  reports every nightly, not only on failures.

## CI

The workflow `.github/workflows/leaner-bench.yaml`:

- **Triggers.** `workflow_dispatch` for the nightly. Repository policy
  prohibits GitHub cron: PIES dispatches the nightly on `main`, registered
  in `internal-ops` as the nightly full suite is
  (`devtools/aptos-cargo-cli/README.md`). Manual dispatch takes
  `problems` (a subset) and `window`; GitHub dispatches only a workflow
  that is on the default branch. A pull request of this repository (not a
  fork) runs the benchmark on its head when it carries the
  `CICD:run-leaner-bench` label, again on every push while it does, and
  its report compares it with `main`'s nightlies.
- **Job `bench`.** The dedicated benchmark runner of the execution
  performance test (`benchmark-c3d-60`), in its concurrency group
  (`execution-benchmark-benchmark-c3d-60`, queued, never cancelled), so that
  no other benchmark shares the machine. Steps: check out the commit; set
  up Rust (`.github/actions/rust-setup`) and Python 3.12; install Lean
  (`scripts/dev_setup.sh -b -p -l -k`); build the ci-profile `move` CLI;
  install the Rust exporter's pinned toolchain; `lake build leaner-bench`
  in `leaner-e2e-tests`; `scripts/leaner-bench.py run`; upload
  `leaner-bench-results`. `timeout-minutes: 150`.
- **Job `report`.** Needs `bench`, on a small runner, with `permissions:
  actions: read` and `GH_TOKEN: ${{ github.token }}` for the history. Runs
  `leaner-bench.py report --current` over the downloaded results of this
  run, which the history lists only once the run has completed; appends
  the markdown to `$GITHUB_STEP_SUMMARY`; and uploads the page and the
  Slack message as `leaner-bench-report`.
- **Job `publish`.** Needs `report`, with `permissions: contents: write`,
  the only job that writes; it runs no code of the benchmarked commit.
  Commits the page to the `gh-pages` branch (a sparse checkout of
  `leaner-bench/`, rebasing and retrying when a concurrent deploy moved
  the branch), and on a dispatch from `main` posts the Slack message with
  `slackapi/slack-github-action` (pinned as in
  `workflow-run-execution-performance.yaml`) to
  `secrets.EXECUTION_PERF_SLACK_WEBHOOK_URL`, the execution performance
  channel. A failed post does not fail the run.
- **Concurrency.** One group per ref, runs never cancelled, so that every
  nightly produces a point.

The suite took about 9 minutes locally; with the builds, the job should
stay well within the hour the set is trimmed to.

## Local use

```bash
cargo build --locked --profile ci -p aptos-move-cli --features binary --bin move
(cd third_party/move/lean/leaner-e2e-tests && lake build leaner-bench)
python3 third_party/move/lean/scripts/leaner-bench.py run --out /tmp/local.json
python3 third_party/move/lean/scripts/leaner-bench.py report \
  --local /tmp/local.json --html /tmp/bench.html
```

`run --package <dir>` takes the executable from another build of
`leaner-e2e-tests`, such as a copy on a native disk.

`run --only ordered_map,math128` measures a subset. `report --local`
fetches the CI history with the user's `gh` login and appends the local run
as a separate point. Its comparison table measures the local run against the
latest CI run at or before the merge base of the local branch with
`main`, so that changes that landed on `main` since do not show up as the
local change. Against CI, the table leads with heartbeats, which compare
directly when the toolchain matches (the report warns when it does not);
wall time differs with the machine, so it is shown as each problem's share
of the suite's time, which a change to one problem moves on any machine.
To measure a change in wall time, `compare` two local runs, before and
after, on the same idle machine.

## Stages

1. **Measurement** (done). Heartbeats in the phase clock; `leaner-bench`
   for the three kinds; CPU time in the driver. Repeats agree to within
   0.001 % in heartbeats.
2. **Module selection** (done). `--modules` in the exporter with the
   closure above, tested in `aptos-move/cli/src/tests/exchange.rs`.
3. **Manifest and driver** (done). `problems.toml` with the initial set,
   `run`, `report`, and `compare`; the suite takes about 9 minutes locally.
4. **CI** (written, not yet run). The workflow with the `bench` job and the
   artifact, run by manual dispatch. Five runs of one commit measure the
   wall-time noise per problem on the runner, which sets the repeat counts
   and the report's threshold.
5. **History and Slack** (written, not yet run). The `report` job, the HTML
   and step summary, and the Slack post; the PIES registration in
   `internal-ops` remains.
6. **Pull requests** (label written, not yet run). The `CICD:run-leaner-bench`
   label runs the benchmark on a pull request, with the comparison in the
   job summary. A sticky comment, as the mono-move benchmark posts, remains
   open.

## Open decisions

- **Runner queue.** The dedicated runner is shared with the execution
  performance and mono-move benchmarks, so the nightly may wait for them.
  If the wait becomes a problem, PIES can dispatch the benchmark at a time
  the others do not run.
- **Slack cadence.** Every nightly, as proposed, or only when a problem
  moves beyond the threshold or changes status.
- **Window.** 30 runs by default.

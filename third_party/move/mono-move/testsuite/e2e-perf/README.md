# MonoMove end-to-end performance

Compares MonoMove against the V1 MoveVM on a full single-node execution
pipeline, sequentially, and reports the speedup per pipeline stage.

## What it measures

For each workload, `run_e2e_perf_test.py`:

1. Creates a warmup DB once (2M accounts by default), shared by every workload.
2. Generates the workload's blocks once and writes them to a file
   (`--dump-blocks`), leaving the initialized DB behind.
3. Replays that file `REPEATS` times per VM, alternating V1 and MonoMove.
4. Takes the median per VM and reports `median(MonoMove) / median(V1)`.

Every workload runs the same block size, `BLOCK_SIZE` (1000). Block overhead is
charged per block, so a smaller block carries more of it per transaction; giving
each workload its own natural batch size would move its speedup for a reason
that has nothing to do with the workload and would make the suite's numbers
incomparable to each other.

The recording step exists because the transaction generators draw from entropy.
Two independent runs of the same workload produce different transactions, and on
workloads whose cost depends on earlier transactions — orderbook, liquidity pool
— that difference shows up as a difference in speed. Replaying one recording on
both VMs removes it.

The two replays differ only in a feature flag override applied after the
workload is initialized: MonoMove gets `--enable-feature-after-init ENABLE_MONO_MOVE`,
V1 gets `--disable-feature-after-init ENABLE_MONO_MOVE`. Both run the same
governance script and the same epoch change, so the only difference between them
is the flag's value.

Initialization always runs on V1. MonoMove discards module-publish payloads,
so a workload that publishes modules could not be set up under it.

The report defines its own columns, verdicts, and workloads in a collapsed block
below the tables. This file covers the method and the calibration.

Ten of the workloads come from `../benches-e2e/`, a set of Move packages shaped
after protocols that run on mainnet. Its README covers what each one stresses
and what a new package there has to satisfy.

## Record and replay

`run-executor` takes two directories, and the record/replay flow is built out of
how they relate:

- `--data-dir` is the database the run starts from. The run only reads it, so
  the same one can feed any number of later runs.
- `--checkpoint-dir` receives a fresh copy of that database at startup.
  Everything the run writes — workload initialization, the feature flag
  override, the executed blocks — lands in the copy.

A recording and its replays chain the two. The recording's `--checkpoint-dir`
becomes each replay's `--data-dir`:

```
warmup DB
   │ --data-dir
   ▼
record ──▶ blocks file
   │ --checkpoint-dir
   ▼
recorded DB
   │ --data-dir, reused by every replay
   ├──▶ V1 replay        ──▶ own throwaway --checkpoint-dir
   └──▶ MonoMove replay  ──▶ own throwaway --checkpoint-dir
```

1. The recording run reads the warmup DB, initializes the workload into its
   checkpoint, writes the generated blocks to a file, and stops. Nothing is
   executed and no feature flag is overridden, so the checkpoint holds exactly
   the state the blocks were generated against. The override belongs to the
   replay, which is what keeps this state neutral between the two VMs.

2. Each replay run reads the recorded DB, overrides `ENABLE_MONO_MOVE` on or
   off, and executes the recorded blocks against its own copy. The copy is
   thrown away afterwards, so every replay starts from the same base and cannot
   see what an earlier one did.

Before overriding, a replay checks that the DB it was handed is at the version,
timestamp, and epoch the recording wrote into the file's header. A mismatch
means the replay was pointed at the wrong directory: sequence numbers would not
line up, and the recorded transactions may already have expired. It fails there
rather than reporting a number.

Only user transactions are recorded. Block metadata carries an epoch and a
timestamp that the override makes stale, so a replay mints a fresh metadata
transaction per block from its own DB.

## What it does not measure

- **Gas.** MonoMove runs unmetered today, so its gas metrics are zero.
- **Parallel execution.** MonoMove is sequential only today.
- **Publish, script, and multisig workloads.** MonoMove discards those payloads.

## Execution and storage measurements

Two tables break the pipeline down further than the stage timers do. The
execution one reads `aptos_executor_other_timers_seconds`, the storage one reads
`aptos_storage_other_timers_seconds`. Each row names a stage in words and prints
the Prometheus label beside it.

The benchmark walks the Prometheus registry rather than a fixed list of labels,
so a timer added anywhere under those two families reaches the report without a
change on the Rust side. What gets a row, a name, and a place in the tree is
decided by `EXECUTION_ROWS` and `STORAGE_ROWS` in `run_e2e_perf_test.py`. A
label with no row still lands in the benchmark's stdout tables and in the JSON
line the harness reads.

Values are milliseconds per block, summed over the median run of every workload
and divided by the blocks those runs covered. It is time summed across threads,
not a share of block latency: several storage stages run concurrently in one
rayon scope, so the children add up past their parent's wall clock. Speedup is
V1 over MonoMove, inverted from the raw times so that above 1.00x still means
MonoMove is faster.

Neither table is calibrated and neither decides a verdict. They say where the
time went once `execution` has already said whether something moved.

## Output size

The report also records bytes written per transaction on each VM. The two need
not agree, and neither direction is a bug on its own:

- MonoMove is unmetered today, so it writes no fee slots and emits no fee
  statement. That pulls its output below V1's.
- MonoMove copies on every `borrow_global_mut`, so a copy counts as a write even
  where the value did not change. That pulls its output above V1's.

Either can change as MonoMove gains metering or a finer write set, so the
absolute numbers are not the point. The ratio is reported, not calibrated; read
it alongside the ledger update and commit columns, which move with it.

## Per-block charts

Set `CHART_DIR` and the harness renders one SVG per workload: every block's
execution, ledger update, and commit time, one panel per VM. In CI the SVGs are
uploaded as the `mono-move-e2e-perf-charts` artifact and the report links to it.

The bars are grouped, never stacked. The four pipeline stages run concurrently,
so their times overlap in wall clock and do not sum to block latency; stacking
them would claim a part-to-whole relationship that does not hold.

Each panel is scaled to its own peak, which is named in its top right. A shared
scale would flatten MonoMove into the baseline, since V1 execution runs several
times longer than anything MonoMove does. What the chart shows is the shape
within one VM: which stage dominates, and whether the tail is a fixed warmup
cost or a recurring stall. That is where to look when end-to-end moves but
execution does not. To compare the two VMs, read the axis rather than the bar
height.

`block_stage_chart.py` writes the SVG by hand and pulls in no dependency. The
CI job installs exactly one Python package and this adds none.

## Running locally

Small and fast, for checking the harness works:

```bash
REPEATS=1 NUM_BLOCKS_PER_TEST=3 NUM_INIT_ACCOUNTS=20000 \
  ONLY_WORKLOADS=no-op,apt-fa-transfer \
  python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
```

Full run, as CI does it:

```bash
RUN_SOURCE=local python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
```

### Environment

| Var | Default | Purpose |
| --- | --- | --- |
| `REPEATS` | `3` | Replays of the recorded blocks per VM |
| `NUM_BLOCKS_PER_TEST` | `30` | Blocks recorded and replayed |
| `NUM_INIT_ACCOUNTS` | `2000000` | Warmup DB size |
| `CREATE_DB_THREADS` | `32` | `create-db` only; measured replays always use 1 thread |
| `BUILD` | `release` | `release` or `performance` (the LTO profile) |
| `ONLY_WORKLOADS` | unset | Comma-separated filter |
| `SELF_COMPARE` | unset | Run V1 against itself; see below |
| `RUN_SOURCE` | `local` | `ci`, `manual`, or `local`; tags the JSON lines |
| `RUNNER_NAME` | `none` | Tags the JSON lines; in CI it also picks the runner |
| `REPORT_PATH` | unset | Write the markdown report here |
| `CHART_DIR` | unset | Write one per-block chart SVG per workload here |
| `RUN_URL` | unset | CI run URL the report links its chart artifact from |
| `WARMUP_BLOCKS` | `2` | Blocks dropped from the head of the steady-state window |
| `HIDE_OUTPUT` | unset | Suppress the benchmark's own log lines |
| `SILENCE_TIMEOUT_SECS` | `1800` | Kill a subprocess that has printed nothing for this long |

`SILENCE_TIMEOUT_SECS` is what keeps one stuck workload from taking the job's
whole timeout. Every command here prints as it goes — the benchmark logs each
block, cargo logs each crate — so going quiet for half an hour means it has
stopped. The longest silence measured across five clean runs is six minutes,
during a single long crate compile, so the default leaves five times that.

A workload killed this way is retried once, because the stall seen so far is in
opening the workload's copy of the warmup DB and says nothing about the
workload itself. A second hang is reported as failed and the run continues with
the next workload. Any other failure — a panic, an abort, a discard — is a real
finding and is never retried.

## Running in CI

`.github/workflows/mono-move-e2e-perf.yaml` does not run on every pull request.
It runs when the PR carries the **`mono-move-e2e-perf`** label. Add the label and
the job starts; it re-runs on each push afterwards, so a fix shows new numbers
without touching the label again. Removing the label stops further runs.

The label only works on same-repo branches. A pull request from a fork is
skipped, because the job builds and runs the PR's code on a self-hosted runner.

Results land in four places: the job's step summary, a sticky PR comment that is
rewritten on each run, one JSON line per calibrated metric in the job log for
Humio to pick up, and the `mono-move-e2e-perf-charts` artifact.

`workflow_dispatch` runs the same job on a branch without a PR, and takes
`REPEATS`, `NUM_BLOCKS_PER_TEST`, `SELF_COMPARE`, `BUILD`, and `RUNNER_NAME` as
inputs. `RUNNER_NAME` picks the runner and tags the samples with it, so the two
can never disagree. A pull request always gets the default, `benchmark-c3d-60`,
which is the machine the calibrated bands were measured on. Dispatching onto a
different runner produces numbers that are not comparable to them.

## Measuring the harness's own noise

`SELF_COMPARE=1` runs V1 on both sides. Every ratio should come out at
1.00x, because both sides replay identical bytes. Whatever it actually comes out
to is the harness's measurement error, and every calibrated band has to sit
above it.

```bash
SELF_COMPARE=1 python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
```

The report prints a noise floor line under the execution table. Run this once per
runner type before trusting any band, and record the result here:

| runner | date | config | largest deviation from 1.00x | largest run-to-run range |
| --- | --- | --- | --- | --- |
| Apple M-series laptop | 2026-09-03 | 5 blocks, 3 repeats, 20k accounts | 0.7% | 3.6% |
| `benchmark-c3d-60` | 2026-09-17 | 30 blocks, 3 repeats, 2M accounts | 1.2% | 2.6% |

Both numbers cover `execution`, which is the only metric a verdict rests on.
Every other metric is disk bound or takes single-digit milliseconds per block,
so its range across two identical runs reaches tens of percent. Those are
reported but neither calibrated nor able to decide a verdict.

A self-compare warns when a ratio lands more than `SELF_COMPARE_MAX_DEVIATION`
(3%) away from 1.00x. Tighten it if a runner turns out to be quieter than that.

## Calibration

Only `execution` is calibrated. It is what verdicts are read from, and it is the
only metric repeatable enough for a band to mean anything: measured across this
file, its mean run-to-run range is 3.0% against 10.3% for end-to-end throughput.
End-to-end tracks whichever pipeline stage is slowest, and MonoMove is fast
enough that the slowest one is commit, so a band on it would be three times
wider without catching anything `execution` would miss.

`e2e_perf_speedup.tsv` holds the calibrated speedups. Columns:

- `workload`, `metric` — the key.
- `num_samples` — how many CI runs went into the row.
- `lowest_over_median` — the smallest observed speedup over the median one.
  `0.951` means the worst run came in 4.9% below the middle one.
- `highest_over_median` — the same for the largest.
- `median_speedup` — the calibrated number. MonoMove throughput over V1
  throughput, not a TPS.

`speedup_band` estimates the run-to-run deviation and allows `BAND_DEVIATIONS`
(3) of it either side of `median_speedup`, plus a `BAND_FLOOR` (3%) floor. The
estimate divides the observed range by the range a normal distribution is
expected to cover in that many samples, so the two spread columns are used only
through their difference and the band comes out symmetric.

Estimating the deviation, rather than scaling the range directly, is what makes
the band independent of the sample count. A wider run set covers a wider range
but is drawn from the same distribution, so the band converges on the real noise
instead of tightening as samples accumulate. More samples make it accurate, not
narrow. Today that lands at roughly ±6% for most workloads.

The floor covers what repeats inside one run cannot see: a workload can be
quiet across five CI runs and still move when the runner is busy. It has to sit
above the self-compare deviation measured on the runner.

The remaining weakness is the range itself: one bad run widens a row's band
until it is re-seeded, which is why `account-generation` sits at ±12.7% while
everything else is near ±6%. Quartiles would be robust to that, but they need
more samples than the five these rows were seeded from to mean anything.

Recalibration rewrites the file as a merge: a row the query did not return keeps
its stored value, so a workload that failed for the whole window does not come
back uncalibrated. Rows are sorted by key, so the diff shows only what moved.

### Bootstrapping, with no history yet

1. `workflow_dispatch` `mono-move-e2e-perf` on your branch 5-6 times.
2. Recalibrate from Humio:
   ```bash
   python3 third_party/move/mono-move/testsuite/e2e-perf/calibrate_e2e_perf_test.py \
     --branch <your-branch> --time-interval 5d
   ```
   Or, if Humio ingestion for the new job name is not working yet, from the
   downloaded job logs:
   ```bash
   python3 third_party/move/mono-move/testsuite/e2e-perf/calibrate_e2e_perf_test.py \
     --from-jsonl run1.log run2.log run3.log
   ```
3. Review the changelog diff and commit both files.

### Steady state

`.github/workflows/calibrate-mono-move-e2e-perf.yaml` runs the calibrator over
the last 60 hours of `main` and opens a PR when something drifts.

## Gating

Every workload starts with `blocking=False`: a regression is reported but does
not fail the job. Flip a workload to `blocking=True` once its band has been
stable for a few days.

The job fails on a `failed` workload regardless. A workload MonoMove cannot
execute at all is a real finding, not noise — the benchmark asserts on aborts,
discards, and retries, so a silent discard panics rather than reporting a fake
speedup.

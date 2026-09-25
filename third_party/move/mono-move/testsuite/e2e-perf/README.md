# MonoMove end-to-end performance

Compares MonoMove against the V1 MoveVM on a full single-node execution
pipeline and reports the speedup per pipeline stage.

The suite runs at two concurrency levels, each with its own CI job, PR label,
and calibration file. `EXECUTION_THREADS=1` measures per-transaction cost.
`EXECUTION_THREADS=16`, the concurrency level a production node runs with, also
carries how well each VM scales. A speedup measured at one level says nothing
about the other, so the two are never compared to each other.

## What it measures

For each workload, `run_e2e_perf_test.py`:

1. Creates a warmup DB once (2M accounts by default), shared by every workload.
2. Generates the workload's blocks once and writes them to a file
   (`--dump-blocks`), leaving the initialized DB behind.
3. Replays that file `REPEATS` times per VM, alternating V1 and MonoMove.
4. Takes the median per VM and reports `median(MonoMove) / median(V1)`.

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

## Concurrency level

`EXECUTION_THREADS` sets `--execution-threads` on every command, so both VMs
run the block executor at the same concurrency level.

MonoMove caps that level at the number of arenas its global context was built
with, and the cap is applied inside the VM. A context sized below the requested
level would turn a parallel run into a near-sequential one that still reports a
speedup. To rule that out, the benchmark logs the level the block executor
actually ran at and the harness fails the workload if any replay came out below
what was asked for. The report states the level every table behind it was
measured at.

## What it does not measure

- **Gas.** MonoMove runs unmetered today, so its gas metrics are zero.
- **Publish, script, and multisig workloads.** MonoMove discards those payloads.

## Output size

The report also records bytes written per transaction on each VM. The two need
not agree, and neither direction is a bug on its own:

- MonoMove is unmetered today, so it writes no fee slots and emits no fee
  statement. That pulls its output below V1's.
- MonoMove copies on every `borrow_global_mut`, so a copy counts as a write even
  where the value did not change. That pulls its output above V1's.

Either can change as MonoMove gains metering or a finer write set, so the
absolute numbers are not the point. The ratio is calibrated, and what the
calibration catches is the ratio moving away from where it was measured.

## Running locally

Small and fast, for checking the harness works:

```bash
REPEATS=1 NUM_BLOCKS_PER_TEST=3 NUM_INIT_ACCOUNTS=20000 \
  ONLY_WORKLOADS=no-op,apt-fa-transfer \
  python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
```

Full run, as the sequential CI job does it:

```bash
RUN_SOURCE=local python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
```

As the parallel CI job does it:

```bash
EXECUTION_THREADS=16 RUN_SOURCE=local \
  python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
```

### Environment

| Var | Default | Purpose |
| --- | --- | --- |
| `REPEATS` | `3` | Replays of the recorded blocks per VM |
| `NUM_BLOCKS_PER_TEST` | `30` | Blocks recorded and replayed |
| `NUM_INIT_ACCOUNTS` | `2000000` | Warmup DB size |
| `EXECUTION_THREADS` | `1` | Block executor concurrency level, both VMs |
| `CREATE_DB_THREADS` | `32` | `create-db` only; the measured replays use `EXECUTION_THREADS` |
| `BUILD` | `release` | `release` or `performance` (the LTO profile) |
| `ONLY_WORKLOADS` | unset | Comma-separated filter |
| `SELF_COMPARE` | unset | Run V1 against itself; see below |
| `RUN_SOURCE` | `local` | `ci`, `manual`, or `local`; tags the JSON lines |
| `RUNNER_NAME` | `none` | Tags the JSON lines; in CI it also picks the runner |
| `REPORT_PATH` | unset | Write the markdown report here |
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

Neither job runs on every pull request. Each runs only when the PR carries its
label:

| workflow | label | threads | calibration |
| --- | --- | --- | --- |
| `.github/workflows/mono-move-e2e-perf.yaml` | `mono-move-e2e-perf` | 1 | `e2e_perf_speedup.tsv` |
| `.github/workflows/mono-move-e2e-perf-parallel.yaml` | `mono-move-e2e-perf-parallel` | 16 | `e2e_perf_speedup_parallel.tsv` |

Add a label and that job starts; it re-runs on each push afterwards, so a fix
shows new numbers without touching the label again. Removing the label stops
further runs. The two are independent: carry one label, the other, or both.

A label only works on same-repo branches. A pull request from a fork is skipped,
because the job builds and runs the PR's code on a self-hosted runner.

Results land in three places: the job's step summary, a sticky PR comment that is
rewritten on each run, and one JSON line per calibrated metric in the job log for
Humio to pick up. The two jobs use different sticky headers, so a PR carrying
both labels ends up with one comment per concurrency level.

`workflow_dispatch` runs either job on a branch without a PR, and takes
`REPEATS`, `NUM_BLOCKS_PER_TEST`, `SELF_COMPARE`, `BUILD`, and `RUNNER_NAME` as
inputs; the parallel one also takes `EXECUTION_THREADS`. `RUNNER_NAME` picks the
runner and tags the samples with it, so the two can never disagree. A pull
request always gets the defaults: `benchmark-c3d-60`, which is the machine the
calibrated bands were measured on, and 16 threads, which is the level the
parallel bands were measured at. Dispatching onto a different runner, or at a
different thread count, produces numbers that are not comparable to them.

## Measuring the harness's own noise

`SELF_COMPARE=1` runs V1 on both sides. Every ratio should come out at
1.00x, because both sides replay identical bytes. Whatever it actually comes out
to is the harness's measurement error, and every calibrated band has to sit
above it.

```bash
SELF_COMPARE=1 python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
```

The report prints a noise floor line under the headline table. Run this once per
runner type and concurrency level before trusting any band, and record the
result here:

| runner | date | config | largest deviation from 1.00x | largest run-to-run range |
| --- | --- | --- | --- | --- |
| Apple M-series laptop | 2026-09-03 | 1 thread, 5 blocks, 3 repeats, 20k accounts | 0.7% | 3.6% |
| `benchmark-c3d-60` | 2026-09-17 | 1 thread, 30 blocks, 3 repeats, 2M accounts | 1.2% | 2.6% |
| `benchmark-c3d-60` | pending | 16 threads, 30 blocks, 3 repeats, 2M accounts | - | - |

Both numbers cover `total`, `execution`, and `inner_block_executor`. The verdict
itself rests on the last two only. Every other stage is disk bound or takes
single-digit milliseconds per block, so its range across two identical runs
reaches tens of percent. Those stages are reported and calibrated but do not
decide a verdict.

Sixteen workers contending over the same slots is noisier than one, so the
parallel floor is expected to sit above the sequential one. Until it is
measured, treat a parallel `regression` verdict as a prompt to re-run rather
than a finding.

A self-compare warns when a ratio lands more than `SELF_COMPARE_MAX_DEVIATION`
(3%) away from 1.00x. Tighten it if a runner turns out to be quieter than that.

## Calibration

`e2e_perf_speedup.tsv` holds the calibrated speedups for the sequential job and
`e2e_perf_speedup_parallel.tsv` for the parallel one. The harness picks the file
from `EXECUTION_THREADS`, and each has its own changelog beside it. Columns:

- `workload`, `metric` — the key.
- `num_samples` — how many CI runs went into the row.
- `lowest_over_median` — the smallest observed speedup over the median one.
  `0.951` means the worst run came in 4.9% below the middle one.
- `highest_over_median` — the same for the largest.
- `median_speedup` — the calibrated number. MonoMove throughput over V1
  throughput, not a TPS.

A verdict is read from the `execution` row. `speedup_band` estimates the
run-to-run deviation and allows `BAND_DEVIATIONS` (3) of it either side of
`median_speedup`, plus a `BAND_FLOOR` (3%) floor. The estimate divides the
observed range by the range a normal distribution is expected to cover in that
many samples, so the two spread columns are used only through their difference
and the band comes out symmetric.

Estimating the deviation, rather than scaling the range directly, is what makes
the band independent of the sample count. A wider run set covers a wider range
but is drawn from the same distribution, so the band converges on the real noise
instead of tightening as samples accumulate. More samples make it accurate, not
narrow. Today that lands at roughly ±6% for most workloads.

The floor covers what repeats inside one run cannot see, and gives a band to a
row whose runs came out identical — every `output_bytes_per_txn` row has a zero
range, since both VMs write deterministically. It has to sit above the
self-compare deviation measured on the runner.

The remaining weakness is the range itself: one bad run widens a row's band
until it is re-seeded, which is why `account-generation` sits at ±12.7% while
everything else is near ±6%. Quartiles would be robust to that, but they need
more samples than the five these rows were seeded from to mean anything.

Recalibration rewrites the file as a merge: a row the query did not return keeps
its stored value, so a workload that failed for the whole window does not come
back uncalibrated. Rows are sorted by key, so the diff shows only what moved.

### Bootstrapping, with no history yet

`e2e_perf_speedup_parallel.tsv` is empty, so every parallel workload reports
`uncalibrated` until it is seeded. `--parallel` picks the parallel job's name
and output file together; without it the calibrator works on the sequential
pair.

1. `workflow_dispatch` `mono-move-e2e-perf` (or `mono-move-e2e-perf-parallel`)
   on your branch 5-6 times.
2. Recalibrate from Humio:
   ```bash
   python3 third_party/move/mono-move/testsuite/e2e-perf/calibrate_e2e_perf_test.py \
     --branch <your-branch> --time-interval 5d [--parallel]
   ```
   Or, if Humio ingestion for the new job name is not working yet, from the
   downloaded job logs:
   ```bash
   python3 third_party/move/mono-move/testsuite/e2e-perf/calibrate_e2e_perf_test.py \
     --from-jsonl run1.log run2.log run3.log [--parallel]
   ```
   The `--from-jsonl` path reads whichever logs it is handed, so do not mix runs
   at different thread counts into one invocation.
3. Review the changelog diff and commit both files.

### Steady state

`.github/workflows/calibrate-mono-move-e2e-perf.yaml` runs the calibrator over
the last 60 hours of `main`, once per concurrency level, and opens a single PR
when something drifts.

## Gating

Every workload starts with `blocking=False`: a regression is reported but does
not fail the job. Flip a workload to `blocking=True` once its band has been
stable for a few days.

The flag lives on the workload, so it gates both jobs. A workload with no
parallel band yet reports `uncalibrated` and cannot regress, so flipping it for
the sequential job is safe until the parallel bands are seeded. After that,
hold both bands stable before flipping.

The job fails on a `failed` workload regardless. A workload MonoMove cannot
execute at all is a real finding, not noise — the benchmark asserts on aborts,
discards, and retries, so a silent discard panics rather than reporting a fake
speedup.

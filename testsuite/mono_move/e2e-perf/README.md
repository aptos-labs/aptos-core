# mono-move end-to-end performance

Compares MonoMove against the legacy MoveVM on a full single-node execution
pipeline, sequentially, and reports the speedup per pipeline stage.

## What it measures

For each workload, `run_e2e_perf_test.py`:

1. Creates a warmup DB once (2M accounts by default), shared by every workload.
2. Generates the workload's blocks once and writes them to a file
   (`--dump-blocks`), leaving the initialized DB behind.
3. Replays that file `REPEATS` times per VM, alternating legacy and MonoMove.
4. Takes the median per VM and reports `median(mono) / median(legacy)`.

The recording step exists because the transaction generators draw from entropy.
Two independent runs of the same workload produce different transactions, and on
workloads whose cost depends on earlier transactions — orderbook, liquidity pool
— that difference shows up as a difference in speed. Replaying one recording on
both VMs removes it.

The two replays differ only in a feature flip applied after the workload is
initialized: MonoMove gets `--enable-feature-after-init ENABLE_MONO_MOVE`,
legacy gets `--disable-feature-after-init ENABLE_MONO_MOVE`. Both run the same
governance script and the same epoch change, so the only difference between them
is the flag's value.

Initialization always runs on legacy. MonoMove discards module-publish payloads,
so a workload that publishes modules could not be set up under it.

## What it does not measure

- **Gas.** MonoMove runs unmetered, so its gas metrics are zero.
- **Parallel execution.** MonoMove is sequential only today.
- **Publish, script, and multisig workloads.** MonoMove discards those payloads.

## Output size

The report also compares bytes written per transaction, which says whether the
two VMs wrote the same thing. MonoMove is unmetered, so it writes no fee slots
and emits no fee statement, and it lands well below 1.00x — around 0.6x on
`apt-fa-transfer` and 0.24x on `no-op`, where the fee is most of the output. The
ratio is calibrated so a change in it shows up. A drop against the calibrated
value means MonoMove skipped real work, and the speedup next to it is not a
speedup.

Pulling the other way, MonoMove overapproximates its write set: a copy on write
counts as a write, even where the value did not change. So the ratio balances the
fee MonoMove skips against the extra slots it reports. On most workloads
the fee dominates and the ratio lands below 1.00x. On
`order-book-no-matches1-market` the overapproximation dominates instead, and the
ratio comes out at 1.00x with MonoMove about 30 bytes per transaction above
legacy. That is expected, not a discrepancy.

It does mean the ratio bounds the real write set from above, so it catches
MonoMove writing too little but says nothing about it writing too much.

## Running locally

Small and fast, for checking the harness works:

```bash
REPEATS=1 NUM_BLOCKS_PER_TEST=3 NUM_INIT_ACCOUNTS=20000 \
  ONLY_WORKLOADS=no-op,apt-fa-transfer \
  python3 testsuite/mono_move/e2e-perf/run_e2e_perf_test.py
```

Full run, as CI does it:

```bash
RUN_SOURCE=local python3 testsuite/mono_move/e2e-perf/run_e2e_perf_test.py
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
| `SELF_COMPARE` | unset | Run legacy against legacy; see below |
| `RUN_SOURCE` | `local` | `ci`, `manual`, or `local`; tags the JSON lines |
| `RUNNER_NAME` | `none` | Tags the JSON lines; bands are machine-specific |
| `REPORT_PATH` | unset | Write the markdown report here |
| `HIDE_OUTPUT` | unset | Suppress the benchmark's own log lines |

## Measuring the harness's own noise

`SELF_COMPARE=1` runs legacy on both sides. Every ratio should come out at
1.00x, because both sides replay identical bytes. Whatever it actually comes out
to is the harness's measurement error, and every calibrated band has to sit
above it.

```bash
SELF_COMPARE=1 python3 testsuite/mono_move/e2e-perf/run_e2e_perf_test.py
```

The report prints a noise floor line under the headline table. Run this once per
runner type before trusting any band, and record the result here:

| runner | date | config | largest deviation from 1.00x | largest spread |
| --- | --- | --- | --- | --- |
| Apple M-series laptop | 2026-09-03 | 5 blocks, 3 repeats, 20k accounts | 0.7% | 3.6% |
| `benchmark-c3d-60` | | | _(not yet measured)_ | |

Both numbers cover `total`, `execution`, and `inner_block_executor`. The verdict
itself rests on the last two only. Every other stage is disk bound or takes
single-digit milliseconds per block, so its run-to-run spread reaches tens of
percent between two identical runs. Those stages are reported and calibrated but
do not decide a verdict.

## Calibration

`e2e_perf_speedup.tsv` holds the calibrated speedups. Columns:

- `workload`, `metric` — the key.
- `num_samples` — how many CI runs went into the row.
- `lowest_over_median` — the smallest observed speedup over the median one.
  `0.951` means the worst run came in 4.9% below the middle one.
- `highest_over_median` — the same for the largest.
- `median_speedup` — the calibrated number. MonoMove throughput over legacy
  throughput, not a TPS.

A verdict is read from the `execution` row. `speedup_band` widens the observed
spread by a factor that shrinks as samples accumulate, so a thinly sampled
workload gets a forgiving band.

### Bootstrapping, with no history yet

1. `workflow_dispatch` `mono-move-e2e-perf` on your branch 5-6 times.
2. Recalibrate from Humio:
   ```bash
   python3 testsuite/mono_move/e2e-perf/calibrate_e2e_perf_test.py \
     --branch <your-branch> --time-interval 5d
   ```
   Or, if Humio ingestion for the new job name is not working yet, from the
   downloaded job logs:
   ```bash
   python3 testsuite/mono_move/e2e-perf/calibrate_e2e_perf_test.py \
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

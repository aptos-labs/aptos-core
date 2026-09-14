# mono-move bench regression gate

Catches regressions in the new VM (the `mono` criterion benches under `../`).
Runs on each PR labeled `mono-move` via
`.github/workflows/mono-move-micro-bench.yaml`.

## How it works

Same-job A/B on a dedicated runner: the benches run on the PR's merge-base with
`main`, then on the PR head compared against it. There is no committed
baseline. `main` is the live baseline, so merging a PR automatically becomes
the baseline the next PR compares against.

Each mono bench is measured three ways, as criterion parameters of one id:

- `<id>/instructions` — retired user-space instructions per iteration. This is
  the gate. The count is a property of the code, not of the machine's mood:
  it does not move with CPU frequency, cache state, co-tenants, or where the
  linker happened to place a function, so it varies by parts per million
  between runs where wall time varies by 10% or more.
- `<id>/time` — wall time. Informational: it is what users feel, but on a
  shared runner it cannot tell a 5% slowdown from noise.
- `<id>/cycles` — CPU cycles. Informational: the same instructions taking more
  cycles points at a memory or branch-prediction effect rather than a code
  change.

criterion reports each metric's change as a 95% confidence interval. A bench
fails the gate only when the whole instruction-count interval exceeds
`threshold_percent` (in `config.json`); if the interval straddles the threshold
the change counts as noise and passes.

What the gate cannot see: a slowdown that keeps the instruction stream
identical, such as a larger cache footprint or a new dependent load. Those show
up only in the time and cycles columns, which is why they stay in the report.

## Requirements

The counters use Linux `perf_event_open` on the calling thread. The runner
needs `kernel.perf_event_paranoid <= 2` (or `CAP_PERFMON`) and, on a virtual
machine, a virtualized PMU. `run.sh` sets `MONO_MOVE_BENCH_REQUIRE_COUNTERS`
so a bench binary aborts, naming the counter and the OS error, instead of
silently gating on nothing. A plain `cargo bench` without that variable skips
the counters with a warning, so the benches still run on macOS and in VMs
without a PMU.

## Files

- `../support/mod.rs` — the criterion `Measurement` over a hardware counter
  and the `bench_main!` macro that runs each mono bench under every metric.
- `compare.py` — reads `target/criterion/<id>/<metric>/change/estimates.json`,
  classifies each mono bench by the gate metric, writes the markdown report,
  exits 1 on a regression.
- `run.sh` — the A/B driver (`ab`) and the noise-floor helper (`calibrate-noise`).
- `config.json` — `gate_metric`, `threshold_percent`, the reported `metrics`,
  and the gated criterion ids.

When you add or rename a mono bench, update `mono_benches` in `config.json`.

## Local use

```bash
# Compare your branch against its merge-base with main (heavy: builds + runs
# both sides; leaves the repo on a detached HEAD).
third_party/move/mono-move/testsuite/benches/perf/run.sh ab origin/main HEAD --out /tmp/report.md

# Measure the machine's noise floor to pick threshold_percent (runs main vs main).
third_party/move/mono-move/testsuite/benches/perf/run.sh calibrate-noise
```

Run from inside the repo. The CI workflow copies these scripts out of the
worktree first, because `ab` checks out other refs.

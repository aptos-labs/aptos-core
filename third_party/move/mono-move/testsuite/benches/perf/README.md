# mono-move bench regression gate

Catches wall-time regressions in the new VM (the `mono` criterion benches under
`../`). Runs on each PR labeled `mono-move` via
`.github/workflows/mono-move-bench.yaml`.

## How it works

Same-job A/B on a dedicated runner: the benches run on the PR's merge-base with
`main`, then on the PR head compared against it. A bench fails the gate only when
the whole 95% CI of its slowdown clears the noise band `T` (`threshold_pct` in
`config.json`, default 3%). Improvements and within-noise changes pass.

There is no committed baseline. `main` is the live baseline, so merging a PR
automatically becomes the baseline the next PR compares against.

## Files

- `compare.py` — reads `target/criterion/<id>/change/estimates.json`, classifies
  each mono bench, writes the markdown report, exits 1 on a regression.
- `run.sh` — the A/B driver (`ab`) and the noise-floor helper (`calibrate-noise`).
- `config.json` — `threshold_pct` and the gated criterion ids.

When you add or rename a mono bench, update `mono_benches` in `config.json`.

## Local use

```bash
# Compare your branch against its merge-base with main (heavy: builds + runs
# both sides; leaves the repo on a detached HEAD).
third_party/move/mono-move/testsuite/benches/perf/run.sh ab origin/main HEAD --out /tmp/report.md

# Measure the machine's noise floor to pick threshold_pct (runs main vs main).
third_party/move/mono-move/testsuite/benches/perf/run.sh calibrate-noise
```

Run from inside the repo. The CI workflow copies these scripts out of the
worktree first, because `ab` checks out other refs.

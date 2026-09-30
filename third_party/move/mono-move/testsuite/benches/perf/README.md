# mono-move bench regression gate

Catches large wall-time regressions in the `mono` criterion benches under `../`. Runs
on PRs labeled `mono-move` (`.github/workflows/mono-move-micro-bench.yaml`).

## How it works

`run.sh` builds the benches at the PR head and at its merge-base with the PR's base
branch, each in a temporary git worktree. It runs both sides as alternating separate
processes and takes each side's median. A bench fails when the PR is more than
`threshold_percent` slower than base; changes beyond `notable_percent` are only
highlighted. The threshold is deliberately wide: the gate targets large regressions,
not small ones.

Three things keep noise below the threshold:

- **Code alignment.** Both sides are built with every function, and every block
  entered only by a jump, aligned to 64 bytes (`-align-all-functions=6`,
  `-align-all-nofallthru-blocks=6`). Otherwise unrelated code changes shift hot code
  across the CPU's fetch boundaries and swing timings by large amounts. Report timings
  therefore differ from production builds; only the base-to-PR change is meaningful.
- **Separate processes.** Timings vary between processes of the same binary, which
  criterion's in-process confidence interval does not capture; the median across
  processes does.
- **Fixed sampling mode.** Gated bench groups use criterion's Flat sampling (the same
  iteration count in every sample). In the default Auto mode, criterion picks Flat or
  Linear (growing iteration counts) per process from a noisy warm-up estimate, and the
  two modes give different timings for the same code, so a bench near the switch point
  could flip modes between processes or sides.

A bench whose own code changed in the PR (`../<bench>.rs`,
`../../src/programs/<bench>.rs`, or its `.move` program) is reported as
`workload changed` and not gated.

## Files

- `run.sh`: the A/B driver.
- `compare.py`: classifies each gated bench and writes the report.
- `config.json`: thresholds, `processes_per_side`, and the gated ids.

Exit codes: `0` pass, `1` regression, `2` tooling failure (invalid config, build or
bench failure, missing or incomplete results). Anything else is an unexpected error.

Gated ids are `<group>/<function>`, where `<group>` is both the criterion group and the
bench file stem (`fib/mono` lives in `../fib.rs`). Update `config.json` when adding or
renaming a bench, and set `.sampling_mode(SamplingMode::Flat)` on its group.

## Local use

```bash
# PR-style comparison against main. Heavy: builds and runs both sides.
third_party/move/mono-move/testsuite/benches/perf/run.sh ab origin/main HEAD --out /tmp/report.md

# A/A run on one commit, to measure the machine's noise.
third_party/move/mono-move/testsuite/benches/perf/run.sh ab HEAD HEAD
```

Only committed code is measured, and your checkout is not switched. Unset `RUSTFLAGS`
and `CARGO_ENCODED_RUSTFLAGS` first. Artifacts go to `target/mono-bench-gate`
(override with `MONO_BENCH_TARGET_DIR`).

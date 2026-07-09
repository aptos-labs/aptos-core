# mono-move-testsuite

End-to-end differential tests for the MonoMove VM: each Move source (or assembly) input runs on both the legacy MoveVM (v1) and MonoMove (v2), and their behavior is compared. Inputs may also pin specializer golden output (bytecode, stackless IR, micro-ops).

## Framework

Tests use `datatest-stable` (a data-driven harness) over the cases under `tests/test_cases/differential/`. Inputs are `.move` (compiled with `move-compiler-v2`) or `.masm` (assembled with `move-asm`), selected by extension.

## Directives

Each input drives the pipeline with `// RUN:` lines:

- `// RUN: publish [--print(<sections>)]` — destack plus per-function micro-op lowering. `--print` renders specializer golden output into the `.exp`; sections are any of `bytecode`, `stackless`, `micro-ops`. A function that cannot be lowered at publish time renders `skipped (<reason>)`.
- `// RUN: execute <addr>::<mod>::<fn> --args ... [--heap-size <n>]` paired with `// CHECK:` / `// CHECK-SUBSTR:` — runs the function on both the legacy MoveVM (v1) and mono-move (v2) and checks they agree (and match the expected output). `--heap-size <n>` sizes the v2 heap in bytes to force garbage collection under allocation pressure (v1 has no such knob and ignores it).
- `// CHECK-GC-COUNT: <n>` — asserts mono-move (v2) ran exactly `n` garbage collections during the preceding `execute`. v2-only (the legacy VM has no GC); pair with `--heap-size` to drive collections deterministically.

## Baseline (Golden) Files

Each input with a `--print` section has a `<name>.exp` baseline. Baselines are verified, or refreshed with `UPBL=1`; updates should be explainable for the change.

## Running Tests

```bash
cargo test -p mono-move-testsuite --test differential          # verify against baselines
UPBL=1 cargo test -p mono-move-testsuite --test differential   # update baselines
```

# mono-move-testsuite

End-to-end differential tests for the MonoMove VM: each Move source (or assembly) input runs on both the V1 VM and MonoMove (v2), and their behavior is compared. Inputs may also pin specializer golden output (bytecode, stackless IR, micro-ops).

## Framework

Tests use `datatest-stable` (a data-driven harness) over the cases under `tests/test_cases/differential/`. Inputs are `.move` (compiled with `move-compiler-v2`) or `.masm` (assembled with `move-asm`), selected by extension.

## Directives

Each input drives the pipeline with `// RUN:` lines:

- `// RUN: publish [--print(<sections>)]` — destack plus per-function micro-op lowering. `--print` renders specializer golden output into the `.exp`; sections are any of `bytecode`, `stackless`, `micro-ops`. A function that cannot be lowered at publish time renders `skipped (<reason>)`.
- `// RUN: execute <addr>::<mod>::<fn> --args ... [--heap-size <n>]` paired with `// CHECK:` / `// CHECK-SUBSTR:` — runs the function on both the V1 VM and mono-move (v2) and checks they agree (and match the expected output). `--heap-size <n>` sizes the v2 heap in bytes to force garbage collection under allocation pressure (v1 has no such knob and ignores it).
- `// CHECK-GC-COUNT: <n>` — asserts mono-move (v2) ran exactly `n` garbage collections during the preceding `execute`. v2-only (the V1 VM has no GC); pair with `--heap-size` to drive collections deterministically.
- `// CHECK-ERROR-PARITY` — asserts both VMs failed with a VM error and that v2's failure, mapped into v1 terms, matches the status code, sub-status, message, and error location v1 reported. Takes no argument: the expected value is v1's actual output. Move aborts are not covered (they carry no VM error to map, and `CHECK:` already compares them).

## Baseline (Golden) Files

Each input with a `--print` section has a `<name>.exp` baseline. Baselines are verified, or refreshed with `UPBL=1`; updates should be explainable for the change.

## Running Tests

```bash
cargo test -p mono-move-testsuite --test differential          # verify against baselines
UPBL=1 cargo test -p mono-move-testsuite --test differential   # update baselines
```

## Transactional Tests

`tests/transactional.rs` runs the compiler-v2 transactional corpus on MonoVM through the shared framework (`move-transactional-test-runner`), against V1-vm's canonical `.exp` baselines, which this suite cannot update. Trial selection comes from `move-transactional-test-matrix`.

A source whose MonoVM output legitimately differs is listed in the matrix crate's `mono_move_divergences` (with a category and reason) and runs against a MonoVM-owned override under `transactional-baselines/<corpus>/<path>.<config>.exp`. The manifest entry authorizes the override: add the entry, then run with `UB=1` to create or refresh the file. Startup rejects override files without an entry or an active trial, entries with no active trial, and overrides identical to the canonical baseline (the divergence has closed; remove both).

```bash
cargo test -p mono-move-testsuite --test transactional          # verify
UB=1 cargo test -p mono-move-testsuite --test transactional     # create or refresh overrides
```

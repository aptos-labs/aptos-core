# specializer

This crate defines a polymorphic stackless execution IR and performs conversion from Move bytecode to the stackless execution IR.
The stackless execution IR is then lowered into monomorphic micro-ops, when all types used in the function are fully concrete and thus type size and layout information is available.

## Goals of converting from Move bytecode to stackless-exec-ir

- eliminate the implicit operand stack (to reduce operand stack traffic to and from locals)
- keep conversion close to linear time
- preserve polymorphism until later just-in-time monomorphization
- make dataflow explicit enough for local optimization and allocation
- remain simple enough that correctness is easy to reason about

## Test Infrastructure

The specializer pipeline is exercised by the **differential tests** in the `mono-move-testsuite` crate (`mono-move/testsuite/tests/differential.rs`).

### Framework

Tests use `datatest-stable` (a data-driven harness, `harness = false` in the testsuite crate's `Cargo.toml`) over the cases under `mono-move/testsuite/tests/test_cases/differential/`. Inputs are `.move` (compiled with `move-compiler-v2`) or `.masm` (assembled with `move-asm`), selected by extension.

### Directives

Each input drives the pipeline with `// RUN:` lines:

- `// RUN: publish [--print(<sections>)]` — destack plus per-function micro-op lowering. `--print` renders specializer golden output into the `.exp`; sections are any of `bytecode`, `stackless`, `micro-ops`. A function that cannot be lowered at publish time renders `skipped (<reason>)`.
- `// RUN: execute <addr>::<mod>::<fn> --args ...` paired with `// CHECK:` / `// CHECK-SUBSTR:` — runs the function on both the legacy MoveVM (v1) and mono-move (v2) and checks they agree (and match the expected output).

## Baseline (Golden) Files

Each input with a `--print` section has a `<name>.exp` baseline. Baselines are verified, or refreshed with `UPBL=1`; updates should be explainable for the change.

## Running Tests

```bash
cargo test -p mono-move-testsuite --test differential          # verify against baselines
UPBL=1 cargo test -p mono-move-testsuite --test differential   # update baselines
```

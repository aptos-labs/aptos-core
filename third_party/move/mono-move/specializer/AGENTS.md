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

The specializer pipeline is exercised by the **snapshot tests** in the `mono-move-testsuite` crate (`mono-move/testsuite/tests/snapshot.rs`).

### Framework

Tests use `datatest-stable` (a data-driven test harness) with `harness = false` in the testsuite crate's `Cargo.toml`. The entry point is `mono-move/testsuite/tests/snapshot.rs`.

## Test Runners

Two runners are registered, one per input format:

- **`masm_runner`** — Takes `.masm` files (Move assembly), assembles them via `move-asm`, then runs `destack` and per-function micro-op lowering.
- **`move_runner`** — Takes `.move` files, compiles them with `move-compiler-v2`, then runs `destack` and micro-op lowering. Move test output additionally includes the disassembled masm for reference.

## Test Cases

Located under `mono-move/testsuite/tests/test_cases/snapshot/`:

- `masm/` — Hand-written Move assembly inputs (`.masm` files).
- `move/` — Move source inputs (`.move` files).

## Baseline (Golden) Files

Each input file has an expected-output baseline:

- `<name>.exp` — Expected output from the pipeline.

Baselines are verified (or auto-updated). To update baselines after intentional output changes, set `UPBL=1` (update baseline env var) and re-run the tests. The updates should be explainable for the given change.

## Running Tests

```bash
cargo test -p mono-move-testsuite --test snapshot          # verify against baselines
UPBL=1 cargo test -p mono-move-testsuite --test snapshot   # update baselines
```

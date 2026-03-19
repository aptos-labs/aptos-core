# stackless-exec-ir-v1

This crate contains the deprecated V1 pipeline for converting Move bytecode to stackless execution IR. It is kept only for comparison with the V2 pipeline in the sibling `stackless-exec-ir` crate.

## Test Infrastructure

Tests use `datatest-stable` with `harness = false`. The test entry point is `tests/testsuite.rs`.

### Test Runners

- **`masm_runner`** — Takes `.masm` files, assembles via `move-asm`, runs V1 pipeline.
- **`move_runner`** — Takes `.move` files, compiles with `move-compiler-v2`, runs V1 pipeline.

### Baseline Files

Each input has a `.v1.exp` baseline. Set `UPBL=1` to update baselines.

### Running Tests

```bash
cargo test -p stackless-exec-ir-v1
UPBL=1 cargo test -p stackless-exec-ir-v1
```

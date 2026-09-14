// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Programs for benchmarking the mono-move runtime.
//!
//! Every module exposes Move source, run through the mono-move pipeline and
//! through the legacy MoveVM. The micro-kernels also carry a native Rust
//! mirror used as a bench control.
//!
//! Sources are referenced in place, never copied. The micro-kernels live under
//! `tests/test_cases/differential/programs/` and double as the differential
//! tests. The larger workloads live under `mono-move/benchmarks/` as standalone
//! Move packages with their own unit tests.

pub mod bounce;
pub mod bst;
pub mod exchange2;
pub mod fib;
pub mod int_arith_loop;
pub mod match_sum;
pub mod merge_sort;
pub mod nested_loop;
pub mod pathtracer;
pub mod queens;
pub mod sieve;
pub mod testing;
pub mod towers;

use move_binary_format::file_format::CompiledModule;

/// Modulus of the LCG output range (also the seed-reduction modulus). Must
/// match the `LCG_MOD` Move `const` in the `merge_sort`/`bst` fixtures.
pub(crate) const LCG_MOD: u64 = 1_000_003;

/// One step of the LCG used to generate deterministic inputs in the native
/// mirrors, kept byte-identical to the same recurrence in the Move fixtures
/// (`x = (x * LCG_MUL + LCG_INC) % LCG_MOD`).
pub(crate) fn lcg_next(x: u64) -> u64 {
    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    (x * LCG_MUL + LCG_INC) % LCG_MOD
}

/// Compile a single-module Move program source (a canonical `.move` fixture)
/// into a `CompiledModule` for the legacy MoveVM bench flavor. Reuses the
/// harness's stdlib-injecting [`crate::compile::compile_move_source`].
pub(crate) fn compile_one(source: &str) -> CompiledModule {
    crate::compile::compile_move_source(source)
        .expect("Move compilation failed")
        .into_iter()
        .next()
        .expect("no module in compiled output")
}

/// Compile a multi-module Move program source into all its modules.
pub(crate) fn compile_all(source: &str) -> Vec<CompiledModule> {
    crate::compile::compile_move_source(source).expect("Move compilation failed")
}

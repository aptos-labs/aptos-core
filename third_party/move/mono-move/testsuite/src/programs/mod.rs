// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Synthetic programs for benchmarking the mono-move runtime.
//!
//! Each module defines a program in two flavors:
//! - **Native Rust** — reference implementation / bench baseline.
//! - **Move source** — run through the mono-move pipeline (and the legacy
//!   MoveVM). The canonical `.move` files under
//!   `tests/test_cases/differential/programs/` double as the correctness
//!   (differential) tests; the benches in `benches/` `include_str!` the same
//!   files and drive them through the shared [`crate::engine`].

pub mod bst;
pub mod fib;
pub mod int_arith_loop;
pub mod match_sum;
pub mod merge_sort;
pub mod nested_loop;
pub mod testing;

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

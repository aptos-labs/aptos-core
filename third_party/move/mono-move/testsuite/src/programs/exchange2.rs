// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Recursive sudoku enumeration, shaped after SPEC CPU2017 548.exchange2.
//! Deep recursion with backtracking. Correctness is covered by the package's
//! own Move unit tests.

/// Canonical Move source, shared with the benchmark package.
pub const SOURCE: &str = include_str!("../../../benchmarks/bench_exchange2/sources/exchange2.move");

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_exchange2() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}

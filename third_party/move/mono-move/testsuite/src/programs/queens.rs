// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! N-queens backtracking — the Are We Fast Yet kernel, read-heavy vectors with
//! heavily biased branches. Correctness is covered by the package's own Move
//! unit tests.

/// Canonical Move source, shared with the benchmark package.
pub const SOURCE: &str = include_str!("../../../benchmarks/bench_queens/sources/queens.move");

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_queens() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}

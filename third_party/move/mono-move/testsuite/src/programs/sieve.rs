// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Sieve of Eratosthenes — the Are We Fast Yet kernel, the only
//! write-dominated one. Correctness is covered by the package's own Move unit
//! tests.

/// Canonical Move source, shared with the benchmark package.
pub const SOURCE: &str = include_str!("../../../benchmarks/bench_sieve/sources/sieve.move");

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_sieve() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}

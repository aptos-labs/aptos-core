// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Towers of Hanoi — the Are We Fast Yet kernel, highest call-to-loop ratio in
//! the suite. Correctness is covered by the package's own Move unit tests.

/// Canonical Move source, shared with the benchmark package.
pub const SOURCE: &str = include_str!("../../../benchmarks/bench_towers/sources/towers.move");

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_towers() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Match sum — O(n) loop with a 4-arm match inside (wide-diamond CFG).
//!
//! Native Rust reference plus the Move source below (run through the pipeline
//! / MoveVM). Correctness is covered by `differential/programs/match_sum.move`.

/// Canonical Move source — the same file the differential test drives.
pub const SOURCE: &str =
    include_str!("../../tests/test_cases/differential/programs/match_sum.move");

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_match_sum(n: u64) -> u64 {
    let mut sum = 0u64;
    let mut i = 0u64;
    while i < n {
        sum += match i % 4 {
            0 => 10,
            1 => 20,
            2 => 30,
            _ => 40,
        };
        i += 1;
    }
    sum
}

// ---------------------------------------------------------------------------
// Move bytecode (for the legacy MoveVM bench flavor)
// ---------------------------------------------------------------------------

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_match_sum() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}

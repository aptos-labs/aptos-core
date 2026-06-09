// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Nested loop — O(n²) iterations with simple number crunching.
//!
//! Useful as a benchmark for loop dispatch overhead. Native Rust reference
//! plus the Move source below (run through the pipeline / MoveVM). Correctness
//! is covered by `differential/programs/nested_loop.move`.

/// Canonical Move source — the same file the differential test drives.
pub const SOURCE: &str =
    include_str!("../../tests/test_cases/differential/programs/nested_loop.move");

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_nested_loop(n: u64) -> u64 {
    let mut sum = 0u64;
    let mut i = 0u64;
    while i < n {
        let mut j = 0u64;
        while j < n {
            sum = std::hint::black_box(sum.wrapping_add(i ^ j));
            j += 1;
        }
        i += 1;
    }
    sum
}

// ---------------------------------------------------------------------------
// Move bytecode (for the legacy MoveVM bench flavor)
// ---------------------------------------------------------------------------

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_nested_loop() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}

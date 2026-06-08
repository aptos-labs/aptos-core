// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Fibonacci — recursive, exponential-time implementation.
//!
//! Two flavors: a native Rust reference (bench baseline) and the Move source
//! below, run through the mono-move pipeline (and MoveVM) rather than as
//! hand-built micro-ops. Correctness is covered by the differential test at
//! `tests/test_cases/differential/programs/fib.move`.

/// Canonical Move source — the same file the differential test drives.
pub const SOURCE: &str = include_str!("../../tests/test_cases/differential/programs/fib.move");

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_fib(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    native_fib(n - 1) + native_fib(n - 2)
}

// ---------------------------------------------------------------------------
// Move bytecode (for the legacy MoveVM bench flavor)
// ---------------------------------------------------------------------------

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_fib() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}

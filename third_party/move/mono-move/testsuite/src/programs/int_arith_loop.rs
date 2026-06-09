// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tight loop of integer arithmetic ops — a microbenchmark comparing the u64
//! specialized fast path (`MulU64Imm` etc.) against the unspecialized per-kind
//! encoding (`IntMul` etc.). The two flavors are now driven by element type in
//! the Move source: the `u64_loop` entry lowers to the specialized variants and
//! the `i64_loop` entry to the unspecialized ones.
//!
//! Native Rust reference plus the Move source (run through the pipeline /
//! MoveVM). Correctness is covered by `differential/programs/int_arith_loop.move`.

/// Rounds per loop iteration (must match the unrolled body in the Move source).
pub const ROUNDS_PER_ITER: usize = 30;

/// Per-round arithmetic constants. `((acc * MUL) + ADD) % MOD`.
pub const MUL: i64 = 31;
pub const ADD: i64 = 17;
pub const MOD: i64 = 1_000_003; // prime, fits well below 2^20

/// Canonical Move source — the same file the differential test drives.
pub const SOURCE: &str =
    include_str!("../../tests/test_cases/differential/programs/int_arith_loop.move");

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_u64_loop(iters: u64) -> u64 {
    let mut acc: u64 = 1;
    let mut i: u64 = 0;
    while i < iters {
        for _ in 0..ROUNDS_PER_ITER {
            acc = ((acc * MUL as u64) + ADD as u64) % MOD as u64;
        }
        i += 1;
    }
    acc
}

pub fn native_i64_loop(iters: u64) -> i64 {
    let mut acc: i64 = 1;
    let mut i: u64 = 0;
    while i < iters {
        for _ in 0..ROUNDS_PER_ITER {
            acc = ((acc * MUL) + ADD) % MOD;
        }
        i += 1;
    }
    acc
}

// ---------------------------------------------------------------------------
// Move bytecode (for the legacy MoveVM bench flavor)
// ---------------------------------------------------------------------------

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_int_arith_loop() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}

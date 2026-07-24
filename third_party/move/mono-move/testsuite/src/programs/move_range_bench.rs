// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `vector::move_range` microbenchmark — a loop that moves a chunk between two
//! vectors and straight back, isolating the op's per-call cost. `shuffle_small`
//! moves a 2-element chunk (dispatch-dominated); `shuffle_large` a 60-element
//! chunk (copy-dominated). Used to compare lowering `move_range` as a native
//! call versus a dedicated micro-op. Correctness is covered by
//! `differential/programs/move_range_bench.move`.

/// Canonical Move source — the same file the differential test drives.
pub const SOURCE: &str =
    include_str!("../../tests/test_cases/differential/programs/move_range_bench.move");

// ---------------------------------------------------------------------------
// Native Rust (bench baseline)
// ---------------------------------------------------------------------------

/// Mirror of the Move `shuffle_*` loop: move `[rp, rp + len)` out of an
/// `n`-element vector into a scratch vector and back, then return the weighted
/// checksum `sum (i + 1) * v[i]`.
fn native_shuffle(n: usize, rp: usize, len: usize, iters: u64) -> u64 {
    let mut from: Vec<u64> = (1..=n as u64).collect();
    let mut to: Vec<u64> = Vec::new();
    for _ in 0..iters {
        let moved: Vec<u64> = from.drain(rp..rp + len).collect();
        to.splice(0..0, moved);
        let moved: Vec<u64> = to.drain(0..len).collect();
        from.splice(rp..rp, moved);
    }
    from.iter()
        .enumerate()
        .map(|(i, &v)| (i as u64 + 1) * v)
        .sum()
}

pub fn native_shuffle_small(iters: u64) -> u64 {
    native_shuffle(8, 3, 2, iters)
}

pub fn native_shuffle_large(iters: u64) -> u64 {
    native_shuffle(64, 2, 60, iters)
}

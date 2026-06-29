// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Merge sort — recursive, O(n log n) with temp-vector merge.
//!
//! Exercises function calls, loops, and vector operations. Native Rust
//! reference plus the Move source (run through the pipeline / MoveVM).
//! Correctness is covered by `differential/programs/merge_sort.move`, whose
//! `sort_checksum` entry builds its input in-Move via an LCG.

/// Canonical Move source — the same file the differential test drives.
pub const SOURCE: &str =
    include_str!("../../tests/test_cases/differential/programs/merge_sort.move");

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_merge_sort(v: &mut [u64]) {
    let len = v.len();
    if len <= 1 {
        return;
    }
    let mid = len / 2;
    native_merge_sort(&mut v[..mid]);
    native_merge_sort(&mut v[mid..]);

    let mut tmp = Vec::with_capacity(len);
    let (mut i, mut j) = (0, mid);
    while i < mid && j < len {
        if v[i] < v[j] {
            tmp.push(v[i]);
            i += 1;
        } else {
            tmp.push(v[j]);
            j += 1;
        }
    }
    tmp.extend_from_slice(&v[i..mid]);
    tmp.extend_from_slice(&v[j..len]);
    v.copy_from_slice(&tmp);
}

/// Native mirror of the Move `sort_checksum` entry: build `n` LCG values,
/// sort, return `sum(i * sorted[i])`. Used as the bench baseline.
pub fn native_sort_checksum(n: u64, seed: u64) -> u64 {
    let mut values: Vec<u64> = Vec::with_capacity(n as usize);
    let mut x = seed % super::LCG_MOD;
    for _ in 0..n {
        x = super::lcg_next(x);
        values.push(x);
    }
    native_merge_sort(&mut values);
    let mut acc = 0u64;
    for i in 0..n {
        acc += i * values[i as usize];
    }
    acc
}

// ---------------------------------------------------------------------------
// Move bytecode (for the legacy MoveVM bench flavor)
// ---------------------------------------------------------------------------

/// Compile the canonical Move source into a `CompiledModule`.
pub fn move_bytecode_merge_sort() -> move_binary_format::file_format::CompiledModule {
    super::compile_one(SOURCE)
}

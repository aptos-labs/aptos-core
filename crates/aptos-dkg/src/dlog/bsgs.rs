// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Baby-step giant-step (BSGS) discrete log over a range.
//!
//! We recover x such that H = x*G, with 0 <= x < range_limit. The baby-step table holds
//! compressed points j*G for j in [0, m). The giant-step loop computes H - i*m*G for i in [0, n),
//! serializes each point, and looks it up in the baby table; a match gives x = i*m + j.
//! Here n = ceil(range_limit / m). Tuning m ≈ sqrt(range_limit) is common in the literature as it
//! balances time and memory, but we leave it as a parameter to the algorithm.
//!
//! We use a batch size for serialization in the giant-step loop. This is a trade-off between
//! the overhead of serializing and deserializing each point, and the overhead of calling
//! `normalize_batch` for each point. We use a default batch size of 2048, but this can be tuned
//! via benchmarks.
//!
//! We use a threshold for the batch size below which we use the original one-at-a-time algorithm
//! (serialize each projective point without batch normalizing). Above it we use batch normalize + serialize.
//! This threshold is also the result of benchmarks.

use ark_ec::CurveGroup;
use ark_serialize::CanonicalSerialize;
use std::collections::HashMap;

/// Default batch size for serialization in the giant-step loop.
/// Benchmarks can be used to tune this (see `benches/bsgs.rs`, `dlog_bsgs_*_batch_size`).
pub const DEFAULT_BSGS_SERIALIZATION_BATCH_SIZE: usize = 2048;

/// Below this batch size we use the original one-at-a-time algorithm (serialize each
/// projective point without batch normalizing). Above it we use batch normalize + serialize.
/// Tune via benchmarks; small batches (2–8) are often slower than 1 due to normalize_batch overhead.
pub const BSGS_BATCH_NORMALIZE_THRESHOLD: usize = 4;

/// Minimum number of targets for which `dlog_vec` uses the batched-across-targets path.
/// Below this, per-target `dlog` is used (slightly faster for 1–3 targets).
pub const BSGS_VEC_BATCHED_MIN_TARGETS: usize = 4;

/// Compute discrete log using baby-step giant-step with a precomputed table.
/// Uses batched serialization in the giant-step loop; see `dlog_with_batch_size` to tune.
///
/// # Arguments
/// - `G`: base of the exponentiation
/// - `H`: target point
/// - `baby_table`: precomputed HashMap from `C.to_compressed()` |---> exponent
/// - `range_limit`: maximum size of the exponent we're trying to obtain. TODO: Change to u64?
//
// TODO:: ensure that G is also the element used to build the baby_table? So turn baby_table into a struct?
#[allow(non_snake_case)]
pub fn dlog<C: CurveGroup>(
    G: C,
    H: C,
    baby_table: &HashMap<Vec<u8>, u64>,
    range_limit: u64,
) -> Option<u64> {
    dlog_with_batch_size(
        G,
        H,
        baby_table,
        range_limit,
        DEFAULT_BSGS_SERIALIZATION_BATCH_SIZE,
    )
}

/// Same as `dlog` but with configurable serialization batch size for the giant-step loop.
/// If `batch_size` is below `BSGS_BATCH_NORMALIZE_THRESHOLD`, the original one-at-a-time
/// algorithm is used (no batch normalize). Otherwise we use batch normalize + serialize.
#[allow(non_snake_case)]
pub fn dlog_with_batch_size<C: CurveGroup>(
    G: C,
    H: C,
    baby_table: &HashMap<Vec<u8>, u64>,
    range_limit: u64,
    batch_size: usize,
) -> Option<u64> {
    let byte_size = G.compressed_size();

    // Baby-step table size m; giant-step count n = ceil(range_limit / m).
    let m = baby_table
        .len()
        .try_into()
        .expect("Table seems rather large");
    let n = range_limit.div_ceil(m);

    // Precompute -m*G so each giant step is one addition: gamma += G_neg_m.
    let G_neg_m = G * -C::ScalarField::from(m);

    let batch_size = batch_size.max(1);

    if batch_size < BSGS_BATCH_NORMALIZE_THRESHOLD {
        // Original one-at-a-time path: serialize each projective point, no batch normalize
        let mut buf = vec![0u8; byte_size];
        let mut gamma = H;
        for i in 0..n {
            gamma.serialize_compressed(&mut buf[..]).unwrap();
            if let Some(&j) = baby_table.get(&buf[..]) {
                // x = i*m + j: giant-step index i, baby-step index j
                return Some(i * m + j);
            }
            gamma += G_neg_m;
        }
        return None;
    }

    // Batched path: build batch, normalize_batch, then serialize and look up each
    let mut buf = vec![0u8; byte_size];
    let mut batch = Vec::with_capacity(batch_size);

    for chunk_start in (0..n).step_by(batch_size) {
        let actual_batch = (n - chunk_start).min(batch_size as u64) as usize;

        // Giant steps for this chunk: gamma = H - (chunk_start + j)*m*G for j = 0..actual_batch
        batch.clear();
        let mut gamma = H + G_neg_m * C::ScalarField::from(chunk_start);
        for _ in 0..actual_batch {
            batch.push(gamma);
            gamma += G_neg_m;
        }

        let normalized = C::normalize_batch(&batch);
        for (j, aff) in normalized.iter().enumerate() {
            aff.serialize_compressed(&mut buf[..]).unwrap();
            if let Some(&baby_j) = baby_table.get(&buf[..]) {
                return Some((chunk_start + j as u64) * m + baby_j);
            }
        }
    }

    None
}

/// Compute discrete logs for multiple targets. Uses batched-across-targets when
/// `H_vec.len() >= BSGS_VEC_BATCHED_MIN_TARGETS` (fewer normalize_batch calls); otherwise
/// one `dlog` per target (lower overhead for 1–3 targets).
#[allow(non_snake_case)]
pub fn dlog_vec<C: CurveGroup>(
    G: C,
    H_vec: &[C],
    baby_table: &HashMap<Vec<u8>, u64>,
    range_limit: u64,
) -> Option<Vec<u64>> {
    if H_vec.len() >= BSGS_VEC_BATCHED_MIN_TARGETS {
        return dlog_vec_batched(G, H_vec, baby_table, range_limit);
    }

    let mut result = Vec::with_capacity(H_vec.len());
    for H in H_vec {
        if let Some(x) = dlog(G, *H, baby_table, range_limit) {
            result.push(x);
        } else {
            return None; // fail early if any element cannot be solved
        }
    }
    Some(result)
}

/// Same as `dlog_vec` but batches across targets: for each chunk of giant steps we compute
/// points for all targets, call `normalize_batch` once, then serialize and lookup. Fewer
/// normalize_batch calls than calling `dlog` per target; may be faster for large H_vec.
#[allow(non_snake_case)]
pub fn dlog_vec_batched<C: CurveGroup>(
    G: C,
    H_vec: &[C],
    baby_table: &HashMap<Vec<u8>, u64>,
    range_limit: u64,
) -> Option<Vec<u64>> {
    dlog_vec_batched_with_batch_size(
        G,
        H_vec,
        baby_table,
        range_limit,
        DEFAULT_BSGS_SERIALIZATION_BATCH_SIZE,
    )
}

/// Batched-across-targets dlog with configurable giant-step batch size.
///
/// For each chunk of giant steps we build one batch containing points for all targets
/// (layout: target 0's points, then target 1's, …), call `normalize_batch` once, then
/// serialize and lookup. This yields ceil(n / batch_size) normalize_batch calls instead of
/// v * ceil(n / batch_size) when calling `dlog` per target.
#[allow(non_snake_case)]
pub fn dlog_vec_batched_with_batch_size<C: CurveGroup>(
    G: C,
    H_vec: &[C],
    baby_table: &HashMap<Vec<u8>, u64>,
    range_limit: u64,
    batch_size: usize,
) -> Option<Vec<u64>> {
    if H_vec.is_empty() {
        return Some(vec![]);
    }

    let byte_size = G.compressed_size();
    let m = baby_table
        .len()
        .try_into()
        .expect("Table seems rather large");
    let n = range_limit.div_ceil(m);
    let G_neg_m = G * -C::ScalarField::from(m);
    let batch_size = batch_size.max(1).max(BSGS_BATCH_NORMALIZE_THRESHOLD);
    let v = H_vec.len();

    let mut result: Vec<Option<u64>> = vec![None; v];
    // Batch holds v * actual_batch points: [target0_step0..target0_stepK, target1_step0.., ...]
    let mut batch = Vec::with_capacity(v * batch_size.min(n as usize));
    let mut buf = vec![0u8; byte_size];

    for chunk_start in (0..n).step_by(batch_size) {
        // Only process targets not yet solved (avoids redundant work and keeps batch smaller).
        let unsolved: Vec<usize> = (0..v).filter(|&i| result[i].is_none()).collect();
        if unsolved.is_empty() {
            break;
        }

        let actual_batch = (n - chunk_start).min(batch_size as u64) as usize;
        batch.clear();

        // Append giant-step points only for unsolved targets (target 0, then 1, … within unsolved).
        for &target_idx in &unsolved {
            let H = H_vec[target_idx];
            let mut gamma = H + G_neg_m * C::ScalarField::from(chunk_start);
            for _ in 0..actual_batch {
                batch.push(gamma);
                gamma += G_neg_m;
            }
        }

        let normalized = C::normalize_batch(&batch);
        // normalized[idx] corresponds to unsolved target (idx / actual_batch), step (idx % actual_batch)
        for (batch_idx, &result_idx) in unsolved.iter().enumerate() {
            for j in 0..actual_batch {
                let idx = batch_idx * actual_batch + j;
                normalized[idx].serialize_compressed(&mut buf[..]).unwrap();
                if let Some(&baby_j) = baby_table.get(&buf[..]) {
                    // Only assign if not already solved (avoids overwriting with a spurious match).
                    if result[result_idx].is_none() {
                        result[result_idx] = Some((chunk_start + j as u64) * m + baby_j);
                    }
                }
            }
        }
    }

    result.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dlog;
    use ark_bn254::G1Projective;
    use ark_ec::PrimeGroup;

    /// Exhaustive check: recover dlog for every x in [0, range_limit) with a small table.
    #[allow(non_snake_case)]
    #[test]
    fn test_bsgs_bn254_exhaustive() {
        let G = G1Projective::generator();
        let range_limit = 1 << 8;

        let baby_table = dlog::table::build::<G1Projective>(G, 1 << 4);

        // Test **all** values of x from 0 to `range_limit - 1`
        for x in 0..range_limit {
            let H = G * ark_bn254::Fr::from(x);

            let recovered = dlog::<G1Projective>(G, H, &baby_table, range_limit)
                .expect("Failed to recover discrete log");

            assert_eq!(recovered, x, "Discrete log mismatch for x = {}", x);
        }
    }

    /// dlog_vec and dlog_vec_batched must agree for 1, 4, and 16 targets (covers both
    /// the per-target and batched paths in dlog_vec).
    #[allow(non_snake_case)]
    #[test]
    fn test_dlog_vec_batched_matches_dlog_vec() {
        let G = G1Projective::generator();
        let range_limit = 1 << 12;
        let baby_table = dlog::table::build::<G1Projective>(G, 1 << 6);

        for num_targets in [1, 4, 16] {
            let xs: Vec<u64> = (0..num_targets)
                .map(|i| (i as u64) * 17 % range_limit)
                .collect();
            let Hs: Vec<G1Projective> = xs.iter().map(|&x| G * ark_bn254::Fr::from(x)).collect();

            let expected = dlog_vec(G, &Hs, &baby_table, range_limit).expect("dlog_vec failed");
            let batched = dlog_vec_batched(G, &Hs, &baby_table, range_limit)
                .expect("dlog_vec_batched failed");
            assert_eq!(
                expected, batched,
                "dlog_vec vs dlog_vec_batched for num_targets={}",
                num_targets
            );
        }
    }
}

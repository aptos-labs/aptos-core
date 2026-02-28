// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
    dlog_with_batch_size(G, H, baby_table, range_limit, DEFAULT_BSGS_SERIALIZATION_BATCH_SIZE)
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

    let m = baby_table
        .len()
        .try_into()
        .expect("Table seems rather large");
    let n = range_limit.div_ceil(m);

    let G_neg_m = G * -C::ScalarField::from(m);

    let batch_size = batch_size.max(1);

    if batch_size < BSGS_BATCH_NORMALIZE_THRESHOLD {
        // Original one-at-a-time path: serialize each projective point, no batch normalize
        let mut buf = vec![0u8; byte_size];
        let mut gamma = H;
        for i in 0..n {
            gamma.serialize_compressed(&mut buf[..]).unwrap();
            if let Some(&j) = baby_table.get(&buf[..]) {
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
/// `H_vec.len() >= BSGS_VEC_BATCHED_MIN_TARGETS` (faster); otherwise one `dlog` per target.
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
    let mut batch = Vec::with_capacity(v * batch_size.min(n as usize));
    let mut buf = vec![0u8; byte_size];

    for chunk_start in (0..n).step_by(batch_size) {
        let actual_batch = (n - chunk_start).min(batch_size as u64) as usize;
        batch.clear();

        for H in H_vec {
            let mut gamma = *H + G_neg_m * C::ScalarField::from(chunk_start);
            for _ in 0..actual_batch {
                batch.push(gamma);
                gamma += G_neg_m;
            }
        }

        let normalized = C::normalize_batch(&batch);
        for (v, res) in result.iter_mut().enumerate() {
            for j in 0..actual_batch {
                let idx = v * actual_batch + j;
                normalized[idx].serialize_compressed(&mut buf[..]).unwrap();
                if let Some(&baby_j) = baby_table.get(&buf[..]) {
                    *res = Some((chunk_start + j as u64) * m + baby_j);
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

    #[allow(non_snake_case)]
    #[test]
    fn test_dlog_vec_batched_matches_dlog_vec() {
        let G = G1Projective::generator();
        let range_limit = 1 << 12;
        let baby_table = dlog::table::build::<G1Projective>(G, 1 << 6);

        for num_targets in [1, 4, 16] {
            let xs: Vec<u64> = (0..num_targets).map(|i| (i as u64) * 17 % range_limit).collect();
            let Hs: Vec<G1Projective> =
                xs.iter().map(|&x| G * ark_bn254::Fr::from(x)).collect();

            let expected = dlog_vec(G, &Hs, &baby_table, range_limit).expect("dlog_vec failed");
            let batched = dlog_vec_batched(G, &Hs, &baby_table, range_limit)
                .expect("dlog_vec_batched failed");
            assert_eq!(expected, batched, "dlog_vec vs dlog_vec_batched for num_targets={}", num_targets);
        }
    }
}

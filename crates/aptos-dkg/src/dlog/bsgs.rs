// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_ec::CurveGroup;
use ark_serialize::CanonicalSerialize;
use std::collections::HashMap;

/// Default batch size for serialization in the giant-step loop.
/// Benchmarks can be used to tune this (see `benches/bsgs.rs`, `dlog_bsgs_*_batch_size`).
pub const DEFAULT_BSGS_SERIALIZATION_BATCH_SIZE: usize = 64;

/// Below this batch size we use the original one-at-a-time algorithm (serialize each
/// projective point without batch normalizing). Above it we use batch normalize + serialize.
/// Tune via benchmarks; small batches (2–8) are often slower than 1 due to normalize_batch overhead.
pub const BSGS_BATCH_NORMALIZE_THRESHOLD: usize = 16;

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

#[allow(non_snake_case)]
pub fn dlog_vec<C: CurveGroup>(
    G: C,
    H_vec: &[C],
    baby_table: &HashMap<Vec<u8>, u64>,
    range_limit: u64,
) -> Option<Vec<u64>> {
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
}

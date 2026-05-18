// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Baby-step giant-step (BSGS) discrete log over a range.
//!
//! We recover x such that H = x*G, with 0 <= x < range_limit. The baby-step table holds
//! compressed points j*G for j in [0, m). The giant-step loop computes H - i*m*G for i in [0, n),
//! normalises, and looks up in the baby table; a match gives x = i*m + j.
//! Here n = ceil(range_limit / m). (The literature sometimes recommends m ≈ sqrt(range_limit), but this
//! is for one-off calculations; we use a fixed table size.)

use super::table::BabyStepTable;
use ark_ec::CurveGroup;

/// Default batch size for the batched rolling algorithm.
pub const DEFAULT_BSGS_SERIALIZATION_BATCH_SIZE: usize = 512;

/// Basic BSGS: one giant-step at a time. Recovers x with H = x*G, 0 <= x < range_limit.
#[allow(non_snake_case)]
pub fn dlog<C: CurveGroup>(tbl: &BabyStepTable<C::Affine>, H: C, range_limit: u64) -> Option<u64> {
    let n = range_limit.div_ceil(tbl.table_size as u64);
    let mut gamma = H;
    for i in 0..n {
        let aff = gamma.into_affine();
        if let Some(j) = tbl.get(&aff) {
            return Some(i * tbl.table_size as u64 + j as u64);
        }
        gamma += C::from(tbl.G_neg_table_size);
    }
    None
}

/// Discrete logs for multiple targets by calling `dlog` on each entry.
#[allow(non_snake_case)]
pub fn dlog_vec<C: CurveGroup>(
    tbl: &BabyStepTable<C::Affine>,
    H_vec: &[C],
    range_limit: u64,
) -> Option<Vec<u64>> {
    // let mut result = Vec::with_capacity(H_vec.len());
    // for H in H_vec {
    //     result.push(dlog(tbl, *H, range_limit)?);
    // }
    // Some(result)
    dlog_vec_batched_rolling_with_batch_size(
        tbl,
        H_vec,
        range_limit,
        DEFAULT_BSGS_SERIALIZATION_BATCH_SIZE,
    )
}

/// Batched rolling BSGS for a single target (no cross-target batching).
/// Uses batch normalisation over giant-step chunks of size up to `batch_size`.
#[allow(non_snake_case)]
fn dlog_batched_rolling_single<C: CurveGroup>(
    tbl: &BabyStepTable<C::Affine>,
    H: C,
    range_limit: u64,
    batch_size: usize,
) -> Option<u64> {
    let n = range_limit.div_ceil(tbl.table_size as u64);
    let mut gamma = H;
    let mut batch = Vec::<C>::new();

    for chunk_start in (0..n).step_by(batch_size) {
        let actual_batch = (n - chunk_start).min(batch_size as u64) as usize;
        batch.clear();
        batch.reserve(actual_batch);

        for _ in 0..actual_batch {
            batch.push(gamma);
            gamma += C::from(tbl.G_neg_table_size);
        }

        let normalized = C::normalize_batch(&batch);
        for j in 0..actual_batch {
            if let Some(baby_j) = tbl.get(&normalized[j]) {
                return Some((chunk_start + j as u64) * tbl.table_size as u64 + baby_j as u64);
            }
        }
    }
    None
}

/// Batches only per target: for each H in H_vec runs the batched rolling algorithm for that
/// target only (no cross-target batching).
#[allow(non_snake_case)]
pub fn dlog_vec_batched<C: CurveGroup>(
    tbl: &BabyStepTable<C::Affine>,
    H_vec: &[C],
    range_limit: u64,
    batch_size: usize,
) -> Option<Vec<u64>> {
    let mut result = Vec::with_capacity(H_vec.len());
    for H in H_vec {
        result.push(dlog_batched_rolling_single(
            tbl,
            *H,
            range_limit,
            batch_size,
        )?);
    }
    Some(result)
}

/// Rolling BSGS with batching across all targets and configurable batch size.
/// For each chunk of giant steps, builds one batch containing points for all (unsolved) targets,
/// then one batch normalise and lookup. For k = 0, b, 2b, …: B has γ per target and step, γ -= m*G;
/// if B[j] is in the baby table for some target, return (k+j)*m + baby_j. Contrast with
/// `dlog_vec_batched`, which runs this per target only (no cross-target batching).
#[allow(non_snake_case)]
pub fn dlog_vec_batched_rolling_with_batch_size<C: CurveGroup>(
    tbl: &BabyStepTable<C::Affine>,
    H_vec: &[C],
    range_limit: u64,
    batch_size: usize,
) -> Option<Vec<u64>> {
    if H_vec.is_empty() {
        return Some(vec![]);
    }

    let n = range_limit.div_ceil(tbl.table_size as u64);
    let number_of_dlogs = H_vec.len();

    let mut result: Vec<Option<u64>> = vec![None; number_of_dlogs];
    let mut unsolved: Vec<usize> = (0..number_of_dlogs).collect();
    let mut gamma_vec: Vec<C> = H_vec.to_vec();
    // Pre-allocate once so we never reallocate in the hot loop.
    let mut batch = Vec::<C>::with_capacity(number_of_dlogs * batch_size);

    for chunk_start in (0..n).step_by(batch_size) {
        if unsolved.is_empty() {
            break;
        }

        let actual_batch = (n - chunk_start).min(batch_size as u64) as usize;
        batch.clear();
        debug_assert!(batch.capacity() >= unsolved.len() * actual_batch);

        for &t in &unsolved {
            let mut g = gamma_vec[t];
            for _ in 0..actual_batch {
                batch.push(g);
                g += C::from(tbl.G_neg_table_size);
            }
            gamma_vec[t] = g;
        }

        let normalized = C::normalize_batch(&batch);
        for (batch_idx, &result_idx) in unsolved.iter().enumerate() {
            if result[result_idx].is_some() {
                continue;
            }
            for j in 0..actual_batch {
                let idx = batch_idx * actual_batch + j;
                if let Some(baby_j) = tbl.get(&normalized[idx]) {
                    result[result_idx] =
                        Some((chunk_start + j as u64) * tbl.table_size as u64 + baby_j as u64);
                    break;
                }
            }
        }
        unsolved.retain(|&r| result[r].is_none());
    }

    result.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dlog;
    use ark_bn254::{G1Affine, G1Projective};
    use ark_ec::{AffineRepr, PrimeGroup};

    /// Exhaustive check: recover dlog for every x in [0, range_limit) with a small table.
    #[allow(non_snake_case)]
    #[test]
    fn test_bsgs_bn254_exhaustive() {
        let G = G1Affine::generator();
        let range_limit = 1 << 8;

        let baby_table = dlog::table::BabyStepTable::new(G, 1 << 4);

        // Test **all** values of x from 0 to `range_limit - 1`
        for x in 0..range_limit {
            let H = G * ark_bn254::Fr::from(x);

            let recovered = dlog::<G1Projective>(&baby_table, H, range_limit)
                .expect("Failed to recover discrete log");

            assert_eq!(recovered, x, "Discrete log mismatch for x = {}", x);
        }
    }

    /// dlog_vec and dlog_vec_batched must agree.
    #[allow(non_snake_case)]
    #[test]
    fn test_dlog_vec_batched_matches_dlog_vec() {
        let G = G1Projective::generator();
        let range_limit = 1 << 12;
        let baby_table = dlog::table::BabyStepTable::new(G.into_affine(), 1 << 6);

        for num_targets in [1, 4, 16] {
            let xs: Vec<u64> = (0..num_targets)
                .map(|i| (i as u64) * 17 % range_limit)
                .collect();
            let Hs: Vec<G1Projective> = xs.iter().map(|&x| G * ark_bn254::Fr::from(x)).collect();

            let expected = dlog_vec(&baby_table, &Hs, range_limit).expect("dlog_vec failed");
            let batched = dlog_vec_batched(
                &baby_table,
                &Hs,
                range_limit,
                DEFAULT_BSGS_SERIALIZATION_BATCH_SIZE,
            )
            .expect("dlog_vec_batched failed");
            assert_eq!(
                expected, batched,
                "dlog_vec vs dlog_vec_batched for num_targets={}",
                num_targets
            );
        }
    }

    /// dlog_vec_batched_rolling_with_batch_size must match dlog_vec for various batch sizes.
    #[allow(non_snake_case)]
    #[test]
    fn test_dlog_vec_batched_rolling_with_batch_size_matches() {
        let G = G1Projective::generator();
        let range_limit = 1 << 12;
        let baby_table = dlog::table::BabyStepTable::new(G.into_affine(), 1 << 6);

        for num_targets in [1, 4, 8] {
            let xs: Vec<u64> = (0..num_targets)
                .map(|i| (i as u64) * 17 % range_limit)
                .collect();
            let Hs: Vec<G1Projective> = xs.iter().map(|&x| G * ark_bn254::Fr::from(x)).collect();

            let expected = dlog_vec(&baby_table, &Hs, range_limit).expect("dlog_vec failed");
            for &batch_size in &[8, 64, 256, 2048] {
                let rolling = dlog_vec_batched_rolling_with_batch_size(
                    &baby_table,
                    &Hs,
                    range_limit,
                    batch_size,
                )
                .expect("dlog_vec_batched_rolling_with_batch_size failed");
                assert_eq!(
                    expected, rolling,
                    "rolling_with_batch_size({}) for num_targets={}",
                    batch_size, num_targets
                );
            }
        }
    }
}

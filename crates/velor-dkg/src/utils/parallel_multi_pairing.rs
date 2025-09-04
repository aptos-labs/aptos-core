// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use blst::blst_fp12;
use blstrs::{Fp12, G1Affine, G2Affine, Gt};
use group::prime::PrimeCurveAffine;
use rayon::{prelude::*, ThreadPool};

/// Computes a multi-pairing $$\prod_{i=1}^n e(a_i, b_i)$$ using multiple threads from `pool`.
pub fn parallel_multi_pairing_slice(
    terms: &[(&G1Affine, &G2Affine)],
    pool: &ThreadPool,
    min_length: usize,
) -> Gt {
    let res = pool.install(|| {
        terms
            .par_iter()
            .with_min_len(min_length)
            .map(|(p, q)| {
                if (p.is_identity() | q.is_identity()).into() {
                    // Define pairing with zero as one, matching what `pairing` does.
                    blst_fp12::default()
                } else {
                    blst_fp12::miller_loop(q.as_ref(), p.as_ref())
                }
            })
            .reduce(|| blst_fp12::default(), |acc, val| acc * val)
    });

    let out = blst_fp12::final_exp(&res);
    Fp12::from(out).into()
}

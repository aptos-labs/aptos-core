// Copyright © Aptos Foundation

use blst::{blst_final_exp, blst_fp12, blst_fp12_mul, blst_fp12_one, blst_miller_loop};
use blstrs::{Fp12, G1Affine, G2Affine, Gt};
use group::prime::PrimeCurveAffine;
use rayon::{prelude::*, ThreadPool};

/// Computes $$\sum_{i=1}^n \textbf{ML}(a_i, b_i)$$ given a series of terms
/// $$(a_1, b_1), (a_2, b_2), ..., (a_n, b_n).$$
pub fn parallel_multi_miller_loop_and_final_exp(
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

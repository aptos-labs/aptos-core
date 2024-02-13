// Copyright © Aptos Foundation

use blst::{blst_final_exp, blst_fp12, blst_fp12_mul, blst_fp12_one, blst_miller_loop};
use blstrs::{Fp12, G1Affine, G2Affine, Gt};
use group::prime::PrimeCurveAffine;
use rayon::prelude::*;

// TODO: Getting this choice to be right might be tricky. Anything between 2 and 5 seems to give 2.5 ms for size-50 batch.
const MIN_PARALLEL_MULTI_PAIRING_SIZE: usize = 4;

/// Computes $$\sum_{i=1}^n \textbf{ML}(a_i, b_i)$$ given a series of terms
/// $$(a_1, b_1), (a_2, b_2), ..., (a_n, b_n).$$
pub fn parallel_multi_miller_loop_and_final_exp(terms: &[(&G1Affine, &G2Affine)]) -> Gt {
    let res = terms
        .par_iter()
        .with_min_len(MIN_PARALLEL_MULTI_PAIRING_SIZE)
        .map(|(p, q)| {
            if (p.is_identity() | q.is_identity()).into() {
                // Define pairing with zero as one, matching what `pairing` does.
                unsafe { *blst_fp12_one() }
            } else {
                unsafe {
                    let mut tmp = blst_fp12::default();
                    blst_miller_loop(&mut tmp, q.as_ref(), p.as_ref());
                    tmp
                }
            }
        })
        .reduce(
            || unsafe { *blst_fp12_one() },
            |mut acc, val| {
                unsafe {
                    blst_fp12_mul(&mut acc, &acc, &val);
                }
                acc
            },
        );

    //     .collect::<Vec<blst_fp12>>();
    //
    // let mut res = unsafe { *blst_fp12_one() };
    // for val in vals {
    //     unsafe { blst_fp12_mul(&mut res, &res, &val); }
    // }

    let mut out = blst_fp12::default();
    unsafe { blst_final_exp(&mut out, &res) };
    Fp12::from(out).into()
}

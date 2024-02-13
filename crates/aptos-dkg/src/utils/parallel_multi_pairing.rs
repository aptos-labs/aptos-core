// Copyright © Aptos Foundation

use blst::{blst_final_exp, blst_fp12, blst_fp12_mul, blst_fp12_one, blst_miller_loop};
use blstrs::{Fp12, G1Affine, G2Affine, Gt};
use group::prime::PrimeCurveAffine;

/// Computes $$\sum_{i=1}^n \textbf{ML}(a_i, b_i)$$ given a series of terms
/// $$(a_1, b_1), (a_2, b_2), ..., (a_n, b_n).$$
pub fn parallel_multi_miller_loop_and_final_exp(terms: &[(&G1Affine, &G2Affine)]) -> Gt {
    let mut res = blst_fp12::default();

    for (i, (p, q)) in terms.iter().enumerate() {
        let mut tmp = blst_fp12::default();

        if (p.is_identity() | q.is_identity()).into() {
            // Define pairing with zero as one, matching what `pairing` does.
            tmp = unsafe { *blst_fp12_one() };
        } else {
            unsafe {
                blst_miller_loop(&mut tmp, q.as_ref(), p.as_ref());
            }
        }

        if i == 0 {
            res = tmp;
        } else {
            unsafe {
                blst_fp12_mul(&mut res, &res, &tmp);
            }
        }
    }

    let mut out = blst_fp12::default();
    unsafe { blst_final_exp(&mut out, &res) };
    Fp12::from(out).into()
}

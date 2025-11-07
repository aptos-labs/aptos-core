// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_ec::CurveGroup;
use std::collections::HashMap;

/// Compute discrete log using baby-step giant-step with a precomputed table
///
/// # Arguments
/// - `G`: base of the exponentiation
/// - `H`: target point
/// - `baby_table`: precomputed HashMap from `C::Affine.to_compressed()` |---> exponent
/// - `m`: size of the baby-step table
#[allow(non_snake_case)]
pub fn dlog<C: CurveGroup>(G: C, H: C, baby_table: &HashMap<Vec<u8>, u32>, m: u32) -> Option<u32> {
    let G_neg_m = G * -C::ScalarField::from(m);

    let mut gamma = H;
    let size = gamma.compressed_size();

    for i in 0..m {
        let mut buf = vec![0u8; size];
        gamma.serialize_compressed(&mut buf[..]).unwrap();

        if let Some(&j) = baby_table.get(&buf) {
            return Some(i * m + j);
        }

        gamma += G_neg_m;
    }

    None
}

#[allow(non_snake_case)]
pub fn dlog_vec<C: CurveGroup>(
    G: C,
    H_vec: &[C],
    baby_table: &HashMap<Vec<u8>, u32>,
    m: u32,
) -> Option<Vec<u32>> {
    let mut result = Vec::with_capacity(H_vec.len());

    for H in H_vec {
        if let Some(x) = dlog(G, *H, baby_table, m) {
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
        let m = 1 << 4;

        let baby_table = dlog::table::build::<G1Projective>(G, m);

        // Test all values of x from 0 to m-1
        for x in 0..m * m {
            let H = G * ark_bn254::Fr::from(x as u32);

            let recovered =
                dlog::<G1Projective>(G, H, &baby_table, m).expect("Failed to recover discrete log");

            assert_eq!(recovered, x, "Discrete log mismatch for x = {}", x);
        }
    }
}

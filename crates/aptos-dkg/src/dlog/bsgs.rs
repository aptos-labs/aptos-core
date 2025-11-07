// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_ec::CurveGroup;
use std::collections::HashMap;

// N should be 48 for Bn254

/// Compute discrete log using baby-step giant-step with a precomputed table
///
/// # Arguments
/// - `G`: base of the exponentiation
/// - `H`: target point
/// - `baby_table`: precomputed HashMap from `C::Affine.to_compressed()` |---> exponent
/// - `m`: size of the baby-step table
#[allow(non_snake_case)]
pub fn baby_step_giant_step<C: CurveGroup, const N: usize>(
    G: C,
    H: C,
    baby_table: &HashMap<[u8; N], u64>,
    m: u64,
) -> Option<u64> {
    let G_neg_m = G * -C::ScalarField::from(m);

    let mut gamma = H;

    for i in 0..m {
        let mut buf = [0u8; N];
        gamma.serialize_compressed(&mut &mut buf[..]).unwrap();

        if let Some(&j) = baby_table.get(&buf) {
            return Some(i * m + j);
        }

        gamma += G_neg_m;
    }

    None
}

/// Build a baby-step table of size `m`
///
/// Returns a HashMap: `C::Affine.to_compressed() |---> exponent`
#[allow(non_snake_case)]
pub fn build_baby_table<C: CurveGroup, const N: usize>(G: C, m: u64) -> HashMap<[u8; N], u64> {
    let mut table = HashMap::with_capacity(m as usize);
    let mut current = C::zero();

    for j in 0..m {
        let mut buf = [0u8; N];
        current.serialize_compressed(&mut &mut buf[..]).unwrap();
        table.insert(buf, j);
        current += G;
    }

    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::G1Projective;
    use ark_ec::PrimeGroup;
    use rand::{thread_rng, Rng};

    const COMPRESSED_SIZE: usize = 48;

    #[allow(non_snake_case)]
    #[test]
    fn test_bsgs_bn254_random() {
        let G = G1Projective::generator();
        let m = 1 << 16;

        let baby_table = build_baby_table::<G1Projective, COMPRESSED_SIZE>(G, m);

        let mut rng = thread_rng();

        // Test 10 random values of x < m*m (to stay within reasonable range)
        for _ in 0..10 {
            let x: u64 = rng.gen_range(0, m * m);
            let H = G * ark_bn254::Fr::from(x);

            let recovered =
                baby_step_giant_step::<G1Projective, COMPRESSED_SIZE>(G, H, &baby_table, m)
                    .expect("Failed to recover discrete log");

            assert_eq!(recovered, x, "Discrete log mismatch for x = {}", x);
        }
    }
}

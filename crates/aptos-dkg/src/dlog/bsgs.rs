// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_ec::CurveGroup;
use std::collections::HashMap;

/// Compute discrete log using baby-step giant-step with a precomputed table
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
    let byte_size = G.compressed_size();

    let m = baby_table
        .len()
        .try_into()
        .expect("Table seems rather large");
    let n = range_limit.div_ceil(m);

    let G_neg_m = G * -C::ScalarField::from(m);

    let mut gamma = H;

    for i in 0..n {
        let mut buf = vec![0u8; byte_size];
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

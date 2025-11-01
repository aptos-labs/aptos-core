// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides some helper functions for arkworks.

pub mod differentiate;
pub mod hashing;
pub mod serialization;
pub mod shamir;
pub mod vanishing_poly;

use ark_ec::AffineRepr;
use ark_ff::{BigInteger, FftField, Field, PrimeField};
use ark_poly::EvaluationDomain;

/// Returns the first `ell` powers of two as scalar field elements, so
/// [1, 2, 4, 8, 16, ..., 2^{ell - 1}]
pub fn powers_of_two<F: Field>(ell: usize) -> Vec<F> {
    (0..ell).map(|j| F::from(1u64 << j)).collect()
}

/// Commit to scalars by multiplying a base group element (in affine representation)
/// with each scalar.
///
/// Equivalent to `[base * s for s in scalars]`.
pub fn commit_to_scalars<P: AffineRepr>(
    commitment_base: &P,
    scalars: &[P::ScalarField],
) -> Vec<P::Group> {
    scalars.iter().map(|s| *commitment_base * s).collect()
}

// TODO: There's probably a better way to do this?
/// Converts a prime field scalar into a `u32`, if possible. Using
/// `PrimeField` because `into_bigint()` needs it for some reason.
pub fn scalar_to_u32<F: PrimeField>(scalar: &F) -> Option<u32> {
    let mut bytes = scalar.into_bigint().to_bytes_le();

    while bytes.last() == Some(&0) {
        bytes.pop();
    }

    if bytes.len() > 4 {
        // More than 4 bytes → cannot fit in u32
        return None;
    }

    // Pad bytes to 4 bytes for u32 conversion
    let mut padded = [0u8; 4];
    padded[..bytes.len()].copy_from_slice(&bytes);

    Some(u32::from_le_bytes(padded))
}

/// Computes all `num_omegas`-th roots of unity in the scalar field, where `num_omegas` must be a power of two.
pub fn compute_roots_of_unity<F: FftField>(num_omegas: usize) -> Vec<F> {
    let eval_dom = ark_poly::Radix2EvaluationDomain::<F>::new(num_omegas)
        .expect("Could not reconstruct evaluation domain");
    eval_dom.elements().collect()
}

#[cfg(test)]
mod test_scalar_to_u32 {
    use super::scalar_to_u32;

    #[test]
    fn test_round_trip_for_valid_values() {
        for i in [0, 1, 42, 255, 65_535, 1_000_000, u32::MAX] {
            let scalar = ark_bn254::Fr::from(i as u64);
            assert_eq!(scalar_to_u32(&scalar), Some(i));
        }
    }
}

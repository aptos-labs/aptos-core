// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module provides some helper functions for arkworks.

pub mod differentiate;
pub mod hashing;
pub mod msm;
pub mod multilinear_poly;
pub mod random;
pub mod scrape;
pub mod serialization;
pub mod shamir;
pub mod vanishing_poly;
pub mod weighted_sum;

use ark_ec::{pairing::Pairing, AffineRepr};
use ark_ff::{BigInteger, FftField, Field, PrimeField};
use ark_poly::EvaluationDomain;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

/// A pair of canonical group generators for a pairing-friendly elliptic curve.
#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupGenerators<E: Pairing> {
    /// The generator of the G₁ group (affine coordinates).
    pub g1: E::G1Affine,
    /// The generator of the G₂ group (affine coordinates).
    pub g2: E::G2Affine,
}

impl<E: Pairing> Default for GroupGenerators<E> {
    fn default() -> Self {
        Self {
            g1: E::G1Affine::generator(),
            g2: E::G2Affine::generator(),
        }
    }
}

/// Returns the first `ell` powers of two as scalar field elements, so
/// [1, 2, 4, 8, 16, ..., 2^{ell - 1}]
/// TODO: Ought to be slightly faster than using `powers()` from `utils`, but haven't tested this
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

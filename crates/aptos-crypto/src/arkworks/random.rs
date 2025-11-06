// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides functions to sample random elements from cryptographic
//! structures such as prime fields and elliptic curve groups; `arkworks` can
//! do this by itself but the point here is to do it with our version of the
//! `rand` crate, which may differ from the version used by `arkworks` and thus
//! would not be accepted directly.

use crate::arkworks::hashing::unsafe_hash_to_affine;
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::PrimeField;
use rand::Rng;

/// DST used for hashing, see `less_insecure_random_point()`
pub const DST_RAND_CORE_HELL: &[u8; 24] = b"APTOS_RAND_CORE_HELL_DST";

/// A version of ark_ff's UniformRand but for older RNGs
pub trait UniformRand {
    /// Securely generate a random instance of self
    fn rand<R: Rng>(rng: &mut R) -> Self;
}

/// Returns a random element given an older RNG as input. Not very secure since the
/// hash function is vulnerable to timing attacks. Probably alright if only used for setups etc?
/// TODO: benchmark this
pub fn less_insecure_random_point<P: AffineRepr, R>(rng: &mut R) -> P
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut rand_seed = [0u8; 256]; // in our blstrs code it says G1_PROJ_NUM_BYTES here, not sure why
    rng.fill(rand_seed.as_mut_slice());

    unsafe_hash_to_affine(rand_seed.as_slice(), DST_RAND_CORE_HELL)
}

/// NOTE: This function is "insecure" in the sense that the caller learns the discrete log of the
/// random point w.r.t. the generator. In many applications, this is not acceptable.
pub fn insecure_random_point<C: CurveGroup, R>(rng: &mut R) -> C
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let r: C::ScalarField = sample_field_element(rng);

    C::generator().mul(r)
}

/// Samples `n` uniformly random elements from the group, but is insecure in the sense
/// that the caller learns the discrete log of the random point.
pub fn insecure_random_points<C: CurveGroup, R>(n: usize, rng: &mut R) -> Vec<C>
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    (0..n).map(|_| insecure_random_point::<C, R>(rng)).collect()
}

/// Samples `n` uniformly random elements from the prime field `F`.
pub fn sample_field_elements<F: PrimeField, R: Rng>(n: usize, rng: &mut R) -> Vec<F> {
    (0..n).map(|_| sample_field_element::<F, R>(rng)).collect()
}

/// Samples a uniformly random element from the prime field `F`, using rejection sampling.
/// Benchmarks suggest it is ~10x faster than the function `scalar_from_uniform_be_bytes()` below.
pub fn sample_field_element<F: PrimeField, R: Rng>(rng: &mut R) -> F {
    loop {
        // Number of bytes needed for F
        let num_bits = F::MODULUS_BIT_SIZE as usize;
        let num_bytes = num_bits.div_ceil(8);

        // Draw enough random bytes to cover the field size
        let mut bytes = vec![0u8; num_bytes];
        rng.fill_bytes(&mut bytes);

        // Interpret as little-endian integer mod p
        if let Some(f) = F::from_random_bytes(&bytes) {
            return f;
        }
    }
}

/// Creates a scalar from a double-sized little-endian byte array by reducing modulo the field order.
pub fn scalar_from_uniform_be_bytes<F: PrimeField, R: Rng>(rng: &mut R) -> F {
    let num_bits = F::MODULUS_BIT_SIZE as usize;
    let num_bytes = num_bits.div_ceil(8);

    let mut bytes = vec![0u8; 2 * num_bytes];
    rng.fill_bytes(&mut bytes);

    F::from_le_bytes_mod_order(&bytes)
}

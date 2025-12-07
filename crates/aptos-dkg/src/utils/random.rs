// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::blstrs::{
    biguint_to_scalar, G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_FIELD_ORDER, SCALAR_NUM_BYTES,
};
/// TODO(Security): This file is a workaround for the `rand_core_hell` issue, briefly described below.
///
/// Ideally, we would write the following sane code:
///
/// ```ignore
/// let mut dk = Scalar::random(rng);
/// while dk.is_zero() {
///     dk = Scalar::random(rng);
/// }
/// ```
///
/// But we can't due to `aptos-crypto`'s dependency on an older version of `rand` and `rand_core`
/// compared to `blstrs`'s dependency.
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
use group::Group;
use num_bigint::BigUint;
use num_integer::Integer;
use std::ops::Mul;

/// Domain-separator for hash-based randomness generation that works around `rand_core_hell`.
pub const DST_RAND_CORE_HELL: &[u8; 24] = b"APTOS_RAND_CORE_HELL_DST";

pub fn random_nonzero_scalar<R>(rng: &mut R) -> Scalar
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    aptos_crypto::blstrs::random_scalar_internal(rng, true)
}

pub fn random_scalar_from_uniform_bytes(bytes: &[u8; 2 * SCALAR_NUM_BYTES]) -> Scalar {
    let bignum = BigUint::from_bytes_le(&bytes[..]);
    let remainder = bignum.mod_floor(&SCALAR_FIELD_ORDER);

    biguint_to_scalar(&remainder)
}

pub fn random_128bit_scalar<R>(rng: &mut R) -> Scalar
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut low_bytes = [0u8; SCALAR_NUM_BYTES / 2];
    rng.fill(&mut low_bytes[..]);

    // Create a 32-byte array (little-endian) by extending the 128-bit number with zeros
    let mut full_bytes = [0u8; SCALAR_NUM_BYTES];

    // Copy the 128-bit random number to the lower half of the 32-byte array
    full_bytes[..SCALAR_NUM_BYTES / 2].copy_from_slice(&low_bytes);

    Scalar::from_bytes_le(&full_bytes).unwrap()
}

/// Returns a random `blstrs::G1Projective` given an older RNG as input.
///
/// Slow: Takes 50 microseconds.
pub fn random_g1_point<R>(rng: &mut R) -> G1Projective
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut rand_seed = [0u8; 2 * G1_PROJ_NUM_BYTES];
    rng.fill(rand_seed.as_mut_slice());

    G1Projective::hash_to_curve(rand_seed.as_slice(), DST_RAND_CORE_HELL, b"G1")
}

/// Returns a random `blstrs::G2Projective` given an older RNG as input.
///
/// Slow: Takes 150 microseconds.
pub fn random_g2_point<R>(rng: &mut R) -> G2Projective
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut rand_seed = [0u8; 2 * G2_PROJ_NUM_BYTES];
    rng.fill(rand_seed.as_mut_slice());

    G2Projective::hash_to_curve(rand_seed.as_slice(), DST_RAND_CORE_HELL, b"G2")
}

/// Returns a random `blstrs::GTProjective` given an older RNG as input.
///
/// Takes 507 microseconds.
///
/// NOTE: This function is "insecure" in the sense that the caller learns the discrete log of the
/// random G_T point w.r.t. the generator. In many applications, this is not acceptable.
pub fn insecure_random_gt_point<R>(rng: &mut R) -> Gt
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let s = aptos_crypto::blstrs::random_scalar(rng);

    // TODO(TestingPerf): Cannot sample more efficiently than this because `fp12::Fp12` is not exposed.
    Gt::generator().mul(s)
}

/// Returns a vector of random `blstrs::Scalar`'s, given an RNG as input.
pub fn random_scalars<R>(n: usize, rng: &mut R) -> Vec<Scalar>
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut v = Vec::with_capacity(n);

    for _ in 0..n {
        v.push(aptos_crypto::blstrs::random_scalar(rng));
    }

    debug_assert_eq!(v.len(), n);

    v
}

/// Returns a vector of random `blstrs::G1Projective`'s, given an RNG as input.
pub fn random_g1_points<R>(n: usize, rng: &mut R) -> Vec<G1Projective>
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut v = Vec::with_capacity(n);

    for _ in 0..n {
        v.push(random_g1_point(rng));
    }

    debug_assert_eq!(v.len(), n);

    v
}

/// Returns a vector of random `blstrs::G2Projective`'s, given an RNG as input.
pub fn random_g2_points<R>(n: usize, rng: &mut R) -> Vec<G2Projective>
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut v = Vec::with_capacity(n);

    for _ in 0..n {
        v.push(random_g2_point(rng));
    }

    debug_assert_eq!(v.len(), n);

    v
}

/// Returns a vector of random `blstrs::GT`'s, given an RNG as input.
///
/// WARNING: Insecure. See `insecure_random_gt_point` comments.
pub fn insecure_random_gt_points<R>(n: usize, rng: &mut R) -> Vec<Gt>
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut v = Vec::with_capacity(n);

    for _ in 0..n {
        v.push(insecure_random_gt_point(rng));
    }

    debug_assert_eq!(v.len(), n);

    v
}

/// Sometimes we will want to generate somewhat random-looking points for benchmarking, so we will
/// use this faster **insecure** function instead.
pub fn insecure_random_g1_points<R>(n: usize, rng: &mut R) -> Vec<G1Projective>
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let point = random_g1_point(rng);
    let shift = random_g1_point(rng);
    let mut acc = point;
    (0..n)
        .map(|_| {
            acc = acc.double() + shift;
            acc
        })
        .collect::<Vec<G1Projective>>()
}

/// Like `insecure_random_g1_points` but for G_2.
pub fn insecure_random_g2_points<R>(n: usize, rng: &mut R) -> Vec<G2Projective>
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let point = random_g2_point(rng);
    let shift = random_g2_point(rng);
    let mut acc = point;
    (0..n)
        .map(|_| {
            acc = acc.double() + shift;
            acc
        })
        .collect::<Vec<G2Projective>>()
}

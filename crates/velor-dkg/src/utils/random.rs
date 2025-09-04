// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

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
/// But we can't due to `velor-crypto`'s dependency on an older version of `rand` and `rand_core`
/// compared to `blstrs`'s dependency.
use crate::{G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_FIELD_ORDER, SCALAR_NUM_BYTES};
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
use group::Group;
use num_bigint::{BigUint, RandBigInt};
use num_integer::Integer;
use num_traits::Zero;
use std::ops::Mul;

/// Domain-separator for hash-based randomness generation that works around `rand_core_hell`.
pub const DST_RAND_CORE_HELL: &[u8; 24] = b"VELOR_RAND_CORE_HELL_DST";

/// Returns a random `blstrs::Scalar` given an older RNG as input.
///
/// Pretty fast: 623 nanosecond / call.
pub fn random_scalar<R>(rng: &mut R) -> Scalar
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    random_scalar_internal(rng, false)
}

pub fn random_nonzero_scalar<R>(rng: &mut R) -> Scalar
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    random_scalar_internal(rng, true)
}

pub fn random_scalar_internal<R>(rng: &mut R, exclude_zero: bool) -> Scalar
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut big_uint;

    loop {
        // NOTE(Alin): This uses rejection-sampling (e.g., https://cs.stackexchange.com/a/2578/54866)
        // An alternative would be to sample twice the size of the scalar field and use
        // `random_scalar_from_uniform_bytes`, but that is actually slower (950ns vs 623ns)
        big_uint = rng.gen_biguint_below(&SCALAR_FIELD_ORDER);

        // Some key material cannot be zero since it needs to have an inverse in the scalar field.
        if !exclude_zero || !big_uint.is_zero() {
            break;
        }
    }

    crate::utils::biguint::biguint_to_scalar(&big_uint)
}

pub fn random_scalar_from_uniform_bytes(bytes: &[u8; 2 * SCALAR_NUM_BYTES]) -> Scalar {
    let bignum = BigUint::from_bytes_le(&bytes[..]);
    let remainder = bignum.mod_floor(&SCALAR_FIELD_ORDER);

    crate::utils::biguint::biguint_to_scalar(&remainder)
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
    let s = random_scalar(rng);

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
        v.push(random_scalar(rng));
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

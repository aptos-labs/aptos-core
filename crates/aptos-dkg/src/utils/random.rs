// Copyright © Aptos Foundation

use crate::{
    DST_RAND_CORE_HELL, G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_FIELD_ORDER, SCALAR_NUM_BYTES,
};
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
use group::Group;
use num_bigint::{BigUint, RandBigInt};
use num_integer::Integer;
use num_traits::Zero;
use std::ops::Mul;

/// Returns a random `blstrs::Scalar` given an older RNG as input.
/// Hacks around the incompatibility of `blstrs`'s `rand_core` dependency (newer) and `aptos_crypto`'s `rand_core` dependency (older).
///
/// Works pretty fast: 1 microsecond / call.
///
/// TODO(Security): The following code pertains to the `rand_core_hell` hack and should be audited.
pub fn random_scalar<R>(rng: &mut R) -> Scalar
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut bytes = [0u8; 2 * SCALAR_NUM_BYTES];
    rng.fill(&mut bytes);

    random_scalar_from_uniform_bytes(&bytes)
}

pub fn random_scalar_from_uniform_bytes(bytes: &[u8; 2 * SCALAR_NUM_BYTES]) -> Scalar {
    let bignum = BigUint::from_bytes_le(&bytes[..]);
    let remainder = bignum.mod_floor(&SCALAR_FIELD_ORDER);

    crate::utils::biguint::biguint_to_scalar(&remainder)
}

/// Like `random_scalar`. Thought it was slower due to the rejection sampling, but it's not.
#[allow(unused)]
fn random_scalar_alternative<R>(rng: &mut R) -> Scalar
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    //
    // TODO(Security): The following code pertains to the `rand_core_hell` hack and should be audited.
    //
    // Ideally, we would write the following sane code. But we can't due to
    // `aptos-crypto`'s dependency on an older version of rand and rand-core than blstrs's version.
    //
    // ```
    // let mut dk = Scalar::random(rng);
    // while dk.is_zero() {
    //     dk = Scalar::random(rng);
    // }
    // ```
    //
    let mut big_uint;

    // The decryption key cannot be zero since it needs to have an inverse in the scalar field.
    loop {
        // TODO(Security): Make sure this correctly uses the RNG to pick the number uniformly in [0, SCALAR_FIELD_ORDER)
        // NOTE(Alin): This uses rejection-sampling, which should be correct (e.g., https://cs.stackexchange.com/a/2578/54866)
        big_uint = rng.gen_biguint_below(&SCALAR_FIELD_ORDER);
        if !big_uint.is_zero() {
            break;
        }
    }

    crate::utils::biguint::biguint_to_scalar(&big_uint)
}

/// Returns a random `blstrs::G1Projective` given an older RNG as input.
/// Hacks around the incompatibility of `blstrs`'s `rand_core` dependency (newer) and `aptos_crypto`'s `rand_core` dependency (older).
///
/// Takes 50 microseconds.
pub fn random_g1_point<R>(rng: &mut R) -> G1Projective
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    // TODO(Security): The following code pertains to the `rand_core_hell` hack and should be audited.
    let mut rand_seed = [0u8; G1_PROJ_NUM_BYTES];

    rng.fill(rand_seed.as_mut_slice());

    //G1Projective::random(&mut rng);

    G1Projective::hash_to_curve(rand_seed.as_slice(), DST_RAND_CORE_HELL, b"G1")
}

/// Returns a random `blstrs::G2Projective` given an older RNG as input.
/// Hacks around the incompatibility of `blstrs`'s `rand_core` dependency (newer) and `aptos_crypto`'s `rand_core` dependency (older).
///
/// Takes 150 microseconds.
pub fn random_g2_point<R>(rng: &mut R) -> G2Projective
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    // TODO(Security): The following code pertains to the `rand_core_hell` hack and should be audited.
    let mut rand_seed = [0u8; G2_PROJ_NUM_BYTES];

    rng.fill(rand_seed.as_mut_slice());

    G2Projective::hash_to_curve(rand_seed.as_slice(), DST_RAND_CORE_HELL, b"G2")
}

/// Returns a random `blstrs::GTProjective` given an older RNG as input.
/// Hacks around the incompatibility of `blstrs`'s `rand_core` dependency (newer) and `aptos_crypto`'s `rand_core` dependency (older).
///
/// Takes 507 microseconds.
///
/// NOTE: This function is "insecure" in the sense that the caller learns the discrete log of the
/// random G_T point w.r.t. the generator. In many applications, this is not acceptable.
pub fn random_gt_point_insecure<R>(rng: &mut R) -> Gt
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let s = random_scalar(rng);

    // TODO(TestPerformance): Cannot sample more efficiently than this because `fp12::Fp12` is not exposed.
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

/// Returns a vector of random `blstrs::GTT`'s, given an RNG as input.
pub fn random_gt_points_insecure<R>(n: usize, rng: &mut R) -> Vec<Gt>
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    let mut v = Vec::with_capacity(n);

    for _ in 0..n {
        v.push(random_gt_point_insecure(rng));
    }

    debug_assert_eq!(v.len(), n);

    v
}

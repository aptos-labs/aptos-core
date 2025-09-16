// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::{
    parallel_multi_pairing::parallel_multi_pairing_slice, random::random_scalar_from_uniform_bytes,
};
use ark_bn254::Fr; // TODO: Move this elsewhere
use ark_ec::AdditiveGroup;
use blstrs::{
    pairing, Bls12, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt,
    Scalar as ScalarOld,
};
use group::{Curve, Group};
use pairing::{MillerLoopResult, MultiMillerLoop};
use rayon::ThreadPool;
use sha3::Digest;
use std::ops::Mul;

pub(crate) mod biguint;
pub mod parallel_multi_pairing;
pub mod random;
pub mod serialization;

#[inline]
pub fn is_power_of_two(n: usize) -> bool {
    n != 0 && (n & (n - 1) == 0)
}

/// Hashes the specified `msg` and domain separation tag `dst` into a `Scalar` by computing a 512-bit
/// number as SHA3-512(SHA3-512(dst) || msg) and reducing it modulo the order of the field.
/// (Same design as in `curve25519-dalek` explained here <https://crypto.stackexchange.com/questions/88002/how-to-map-output-of-hash-algorithm-to-a-finite-field>)
///
/// NOTE: Domain separation from other SHA3-512 calls in our system is left up to the caller.
pub fn hash_to_scalar(msg: &[u8], dst: &[u8]) -> ScalarOld {
    // First, hash the DST as `dst_hash = H(dst)`
    let mut hasher = sha3::Sha3_512::new();
    hasher.update(dst);
    let binding = hasher.finalize();
    let dst_hash = binding.as_slice();

    // Second, hash the msg as `H(dst_hash, msg)`
    let mut hasher = sha3::Sha3_512::new();
    hasher.update(dst_hash);
    hasher.update(msg);
    let binding = hasher.finalize();
    let bytes = binding.as_slice();

    assert_eq!(bytes.len(), 64);

    match bytes.try_into() {
        Ok(chunk) => random_scalar_from_uniform_bytes(chunk),
        Err(_) => panic!("Expected a 64-byte SHA3-512 hash, but got a different size"),
    }
}

/// Works around the `blst_hell` bug (see README.md).
pub fn g1_multi_exp(bases: &[G1Projective], scalars: &[ScalarOld]) -> G1Projective {
    if bases.len() != scalars.len() {
        panic!(
            "blstrs's multiexp has heisenbugs when the # of bases != # of scalars ({} != {})",
            bases.len(),
            scalars.len()
        );
    }

    match bases.len() {
        0 => G1Projective::identity(),
        1 => bases[0].mul(scalars[0]),
        _ => G1Projective::multi_exp(bases, scalars),
    }
}

/// Works around the `blst_hell` bug (see README.md).
pub fn g2_multi_exp(bases: &[G2Projective], scalars: &[ScalarOld]) -> G2Projective {
    if bases.len() != scalars.len() {
        panic!(
            "blstrs's multiexp has heisenbugs when the # of bases != # of scalars ({} != {})",
            bases.len(),
            scalars.len()
        );
    }
    match bases.len() {
        0 => G2Projective::identity(),
        1 => bases[0].mul(scalars[0]),
        _ => G2Projective::multi_exp(bases, scalars),
    }
}

pub fn multi_pairing<'a, I1, I2>(lhs: I1, rhs: I2) -> Gt
where
    I1: Iterator<Item = &'a G1Projective>,
    I2: Iterator<Item = &'a G2Projective>,
{
    let res = <Bls12 as MultiMillerLoop>::multi_miller_loop(
        lhs.zip(rhs)
            .map(|(g1, g2)| (g1.to_affine(), G2Prepared::from(g2.to_affine())))
            .collect::<Vec<(G1Affine, G2Prepared)>>()
            .iter()
            .map(|(g1, g2)| (g1, g2))
            .collect::<Vec<(&G1Affine, &G2Prepared)>>()
            .as_slice(),
    );

    res.final_exponentiation()
}

pub fn parallel_multi_pairing<'a, I1, I2>(
    lhs: I1,
    rhs: I2,
    pool: &ThreadPool,
    min_length: usize,
) -> Gt
where
    I1: Iterator<Item = &'a G1Projective>,
    I2: Iterator<Item = &'a G2Projective>,
{
    parallel_multi_pairing_slice(
        lhs.zip(rhs)
            .map(|(g1, g2)| (g1.to_affine(), g2.to_affine()))
            .collect::<Vec<(G1Affine, G2Affine)>>()
            .iter()
            .map(|(g1, g2)| (g1, g2))
            .collect::<Vec<(&G1Affine, &G2Affine)>>()
            .as_slice(),
        pool,
        min_length,
    )
}

/// Useful for macro'd WVUF code (because blstrs was not written with generics in mind...).
pub fn multi_pairing_g1_g2<'a, I1, I2>(lhs: I1, rhs: I2) -> Gt
where
    I1: Iterator<Item = &'a G1Projective>,
    I2: Iterator<Item = &'a G2Projective>,
{
    multi_pairing(lhs, rhs)
}

/// Useful for macro'd WVUF code (because blstrs was not written with generics in mind...).
pub fn multi_pairing_g2_g1<'a, I1, I2>(lhs: I1, rhs: I2) -> Gt
where
    I1: Iterator<Item = &'a G2Projective>,
    I2: Iterator<Item = &'a G1Projective>,
{
    multi_pairing(rhs, lhs)
}

/// Useful for macro'd WVUF code (because blstrs was not written with generics in mind...).
pub fn pairing_g1_g2(lhs: &G1Affine, rhs: &G2Affine) -> Gt {
    pairing(lhs, rhs)
}

/// Useful for macro'd WVUF code (because blstrs was not written with generics in mind...).
pub fn pairing_g2_g1(lhs: &G2Affine, rhs: &G1Affine) -> Gt {
    pairing(rhs, lhs)
}

pub trait HasMultiExp: for<'a> Sized + Clone {
    fn multi_exp_slice(bases: &[Self], scalars: &[ScalarOld]) -> Self;

    fn multi_exp_iter<'a, 'b, I>(bases: I, scalars: impl Iterator<Item = &'b ScalarOld>) -> Self
    where
        I: Iterator<Item = &'a Self>,
        Self: 'a,
    {
        // TODO(Perf): blstrs does not work with iterators, which leads to unnecessary cloning here.
        Self::multi_exp_slice(
            bases.cloned().collect::<Vec<Self>>().as_slice(),
            scalars.cloned().collect::<Vec<ScalarOld>>().as_slice(),
        )
    }
}

impl HasMultiExp for G2Projective {
    fn multi_exp_slice(points: &[Self], scalars: &[ScalarOld]) -> Self {
        g2_multi_exp(points, scalars)
    }
}

impl HasMultiExp for G1Projective {
    fn multi_exp_slice(points: &[Self], scalars: &[ScalarOld]) -> Self {
        g1_multi_exp(points, scalars)
    }
}

/// Pads the given vector with zeros so that `(len + 1)` becomes the next power of two.
///
/// For example:
/// - If `scalars.len() == 3`, then `len + 1 = 4`, already a power of two,
///   so the vector is padded to length 3 (no change).
/// - If `scalars.len() == 5`, then `len + 1 = 6`, next power of two is 8,
///   so the vector is padded to length 7.
pub(crate) fn pad_to_pow2_len_minus_one(mut scalars: Vec<Fr>) -> Vec<Fr> {
    let target_len = (scalars.len() + 1).next_power_of_two() - 1;
    scalars.resize(target_len, Fr::ZERO);
    scalars
}

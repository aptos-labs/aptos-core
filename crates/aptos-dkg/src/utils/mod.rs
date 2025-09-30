// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::{
    parallel_multi_pairing::parallel_multi_pairing_slice, random::random_scalar_from_uniform_bytes,
};
use ark_ec::{pairing::Pairing, AffineRepr};
use ark_ff::PrimeField;
use blstrs::{pairing, Bls12, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt};
use group::{Curve, Group};
use num_traits::{One, Zero};
use pairing::{MillerLoopResult, MultiMillerLoop};
use rayon::ThreadPool;
use sha3::Digest;
use std::ops::{Mul, MulAssign};

pub(crate) mod biguint;
pub mod parallel_multi_pairing;
pub mod random;
pub mod serialization;
pub mod test_utils;

#[inline]
pub fn is_power_of_two(n: usize) -> bool {
    n != 0 && (n & (n - 1) == 0)
}

pub(crate) fn scalar_to_bits_le<E: Pairing>(x: &E::ScalarField) -> Vec<bool> {
    let bigint: <E::ScalarField as ark_ff::PrimeField>::BigInt = x.into_bigint();
    ark_ff::BitIteratorLE::new(&bigint).collect()
}

/// Hashes the specified `msg` and domain separation tag `dst` into a `Scalar` by computing a 512-bit
/// number as SHA3-512(SHA3-512(dst) || msg) and reducing it modulo the order of the field.
/// (Same design as in `curve25519-dalek` explained here <https://crypto.stackexchange.com/questions/88002/how-to-map-output-of-hash-algorithm-to-a-finite-field>)
///
/// NOTE: Domain separation from other SHA3-512 calls in our system is left up to the caller.
pub fn hash_to_scalar(msg: &[u8], dst: &[u8]) -> blstrs::Scalar {
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
pub fn g1_multi_exp(bases: &[G1Projective], scalars: &[blstrs::Scalar]) -> G1Projective {
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
pub fn g2_multi_exp(bases: &[G2Projective], scalars: &[blstrs::Scalar]) -> G2Projective {
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
    fn multi_exp_slice(bases: &[Self], scalars: &[blstrs::Scalar]) -> Self;

    fn multi_exp_iter<'a, 'b, I>(
        bases: I,
        scalars: impl Iterator<Item = &'b blstrs::Scalar>,
    ) -> Self
    where
        I: Iterator<Item = &'a Self>,
        Self: 'a,
    {
        // TODO(Perf): blstrs does not work with iterators, which leads to unnecessary cloning here.
        Self::multi_exp_slice(
            bases.cloned().collect::<Vec<Self>>().as_slice(),
            scalars.cloned().collect::<Vec<blstrs::Scalar>>().as_slice(),
        )
    }
}

impl HasMultiExp for G2Projective {
    fn multi_exp_slice(points: &[Self], scalars: &[blstrs::Scalar]) -> Self {
        g2_multi_exp(points, scalars)
    }
}

impl HasMultiExp for G1Projective {
    fn multi_exp_slice(points: &[Self], scalars: &[blstrs::Scalar]) -> Self {
        g1_multi_exp(points, scalars)
    }
}

pub(crate) fn msm_bool<G: AffineRepr>(bases: &[G], scalars: &[bool]) -> G::Group {
    assert_eq!(bases.len(), scalars.len());

    let mut acc = G::Group::zero();
    for (base, &bit) in bases.iter().zip(scalars) {
        if bit {
            acc += base;
        }
    }
    acc
}

pub fn powers<T>(base: T, count: usize) -> Vec<T>
where
    T: MulAssign + One + Copy,
{
    let mut powers = Vec::with_capacity(count);
    let mut current = T::one();

    for _ in 0..count {
        powers.push(current);
        current *= base;
    }

    powers
}

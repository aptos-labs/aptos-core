// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod evaluation_domain;
pub mod fft;
pub mod lagrange;
pub mod polynomials;
pub mod threshold_config;
pub mod weighted_config;
pub mod random;
pub mod scalar_secret_key;

use ark_ec::{pairing::Pairing, AffineRepr};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use crate::CryptoMaterialError;
use blstrs::{Bls12, G1Affine, G1Projective, G2Prepared, G2Projective, Gt, Scalar};
use ff::Field;
use group::Curve;
use num_bigint::{BigUint, RandBigInt};
use num_integer::Integer;
use num_traits::Zero;
use once_cell::sync::Lazy;
use pairing::{MillerLoopResult, MultiMillerLoop};


#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct GroupGenerators<E: Pairing> {
    pub g1: E::G1Affine,
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


/// The size in bytes of a scalar.
pub const SCALAR_NUM_BYTES: usize = 32;

/// The size in bytes of a compressed G1 point (efficiently deserializable into projective coordinates)
pub const G1_PROJ_NUM_BYTES: usize = 48;

/// The size in bytes of a compressed G2 point (efficiently deserializable into projective coordinates)
pub const G2_PROJ_NUM_BYTES: usize = 96;

/// The order of the BLS12-381 scalar field as a BigUint
pub static SCALAR_FIELD_ORDER: Lazy<BigUint> = Lazy::new(get_scalar_field_order_as_biguint);

/// Returns the order of the scalar field in our implementation's choice of an elliptic curve group.
pub(crate) fn get_scalar_field_order_as_biguint() -> BigUint {
    let r = BigUint::from_bytes_be(
        hex::decode("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001")
            .unwrap()
            .as_slice(),
    );

    // Here, we paranoically assert that r is correct, by checking 0 - 1 mod r (computed via Scalar) equals r-1 (computed from the constant above)
    let minus_one = Scalar::ZERO - Scalar::ONE;
    let max = &r - 1u8;
    assert_eq!(
        minus_one.to_bytes_le().as_slice(),
        max.to_bytes_le().as_slice()
    );

    r
}

/// Converts a BigUint to a scalar, asserting it fits, panicking otherwise.
///
/// Useful when picking a random scalar and when hashing a message into a scalar.
pub fn biguint_to_scalar(big_uint: &BigUint) -> Scalar {
    // `blstrs`'s `Scalar::from_bytes_le` needs `SCALAR_NUM_BYTES` bytes. The current
    // implementation of `BigUint::to_bytes_le()` does not always return `SCALAR_NUM_BYTES` bytes
    // when the integer is smaller than 32 bytes. So we have to pad it.
    let mut bytes = big_uint.to_bytes_le();

    while bytes.len() < SCALAR_NUM_BYTES {
        bytes.push(0u8);
    }

    debug_assert_eq!(BigUint::from_bytes_le(&bytes), *big_uint);

    let slice = match <&[u8; SCALAR_NUM_BYTES]>::try_from(bytes.as_slice()) {
        Ok(slice) => slice,
        Err(_) => {
            panic!(
                "WARNING: Got {} bytes instead of {SCALAR_NUM_BYTES} (i.e., got {})",
                bytes.as_slice().len(),
                big_uint
            );
        },
    };

    Scalar::from_bytes_le(slice)
        .expect("Deserialization of randomly-generated num_bigint::BigUint failed.")
}

/// Creates a scalar from a big-endian array of bytes, by reducing the number modulo the scalar field order.
///
/// WARNING: Proceed with caution, if using this to sample a random scalar from a seed, then the
/// number of bytes should >= 2 * SCALAR_NUM_BYTES.
pub fn scalar_from_uniform_be_bytes(bytes: &[u8]) -> Scalar {
    let bignum = BigUint::from_bytes_be(bytes);
    let remainder = bignum.mod_floor(&SCALAR_FIELD_ORDER);

    biguint_to_scalar(&remainder)
}

/// Helper method to *securely* parse a sequence of bytes into a `G1Projective` point.
/// NOTE: This function will check for prime-order subgroup membership in $\mathbb{G}_1$.
pub fn g1_proj_from_bytes(bytes: &[u8]) -> Result<G1Projective, CryptoMaterialError> {
    let slice = match <&[u8; G1_PROJ_NUM_BYTES]>::try_from(bytes) {
        Ok(slice) => slice,
        Err(_) => return Err(CryptoMaterialError::WrongLengthError),
    };

    let a = G1Projective::from_compressed(slice);

    if a.is_some().unwrap_u8() == 1u8 {
        Ok(a.unwrap())
    } else {
        Err(CryptoMaterialError::DeserializationError)
    }
}

/// Helper method to *securely* parse a sequence of bytes into a `G2Projective` point.
/// NOTE: This function will check for prime-order subgroup membership in $\mathbb{G}_2$.
pub fn g2_proj_from_bytes(bytes: &[u8]) -> Result<G2Projective, CryptoMaterialError> {
    let slice = match <&[u8; G2_PROJ_NUM_BYTES]>::try_from(bytes) {
        Ok(slice) => slice,
        Err(_) => return Err(CryptoMaterialError::WrongLengthError),
    };

    let a = G2Projective::from_compressed(slice);

    if a.is_some().unwrap_u8() == 1u8 {
        Ok(a.unwrap())
    } else {
        Err(CryptoMaterialError::DeserializationError)
    }
}

/// Helper method to *securely* parse a sequence of bytes into a `Scalar`.
pub fn scalar_from_bytes_le(bytes: &[u8]) -> Result<Scalar, CryptoMaterialError> {
    let slice = match <&[u8; SCALAR_NUM_BYTES]>::try_from(bytes) {
        Ok(slice) => slice,
        Err(_) => return Err(CryptoMaterialError::WrongLengthError),
    };

    let opt = Scalar::from_bytes_le(slice);
    if opt.is_some().unwrap_u8() == 1u8 {
        Ok(opt.unwrap())
    } else {
        Err(CryptoMaterialError::DeserializationError)
    }
}

/// Computes a multi-pairing.
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

/// Returns a random `blstrs::Scalar`.
///
/// Pretty fast: 623 nanosecond / call.
pub fn random_scalar<R>(rng: &mut R) -> Scalar
where
    R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
{
    random_scalar_internal(rng, false)
}

/// Returns a random `blstrs::Scalar`, optionally restricted to be non-zero.
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

    biguint_to_scalar(&big_uint)
}

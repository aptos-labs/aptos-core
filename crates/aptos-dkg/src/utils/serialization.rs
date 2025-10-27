// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_NUM_BYTES};
use aptos_crypto::CryptoMaterialError;
use blstrs::{G1Projective, G2Projective, Scalar};
use ark_serialize::CanonicalSerialize;
use ark_serialize::Compress;
use ark_serialize::Validate;
use ark_serialize::CanonicalDeserialize;

pub fn ark_se<S, A: CanonicalSerialize>(a: &A, s: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
      let mut bytes = vec![];
      a.serialize_with_mode(&mut bytes, Compress::No).map_err(serde::ser::Error::custom)?;
      s.serialize_bytes(&bytes)
}

pub fn ark_de<'de, D, A: CanonicalDeserialize>(data: D) -> Result<A, D::Error> where D: serde::de::Deserializer<'de> {
      let s: Vec<u8> = serde::de::Deserialize::deserialize(data)?;
      let a = A::deserialize_with_mode(s.as_slice(), Compress::No, Validate::No);
      a.map_err(serde::de::Error::custom)
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
pub(crate) fn scalar_from_bytes_le(bytes: &[u8]) -> Result<Scalar, CryptoMaterialError> {
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

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_NUM_BYTES};
use velor_crypto::CryptoMaterialError;
use blstrs::{G1Projective, G2Projective, Scalar};

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

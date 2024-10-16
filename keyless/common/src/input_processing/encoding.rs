// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::api::{EphemeralPublicKeyBlinder, PoseidonHash};
use anyhow::{anyhow, Result};
use aptos_types::{
    jwks::rsa::RSA_JWK, keyless::Pepper, transaction::authenticator::EphemeralPublicKey,
};
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

pub type RsaSignature = BigUint;

pub trait AsFr {
    fn as_fr(&self) -> Fr;
}

pub trait FromFr {
    fn from_fr(fr: &Fr) -> Self;
}

pub trait TryFromFr: Sized {
    fn try_from_fr(fr: &Fr) -> Result<Self>;
}

impl AsFr for PoseidonHash {
    fn as_fr(&self) -> Fr {
        Fr::from_le_bytes_mod_order(self.as_slice())
    }
}

impl TryFromFr for PoseidonHash {
    fn try_from_fr(fr: &Fr) -> Result<Self> {
        let v = fr.into_bigint().to_bytes_le();
        let arr: PoseidonHash = v
            .try_into()
            .map_err(|_| anyhow!("Conversion from Fr to bytes failed"))?;
        Ok(arr)
    }
}

impl AsFr for EphemeralPublicKeyBlinder {
    fn as_fr(&self) -> Fr {
        Fr::from_le_bytes_mod_order(self)
    }
}

impl FromFr for EphemeralPublicKeyBlinder {
    fn from_fr(fr: &Fr) -> Self {
        fr.into_bigint().to_bytes_le()
    }
}

impl AsFr for Pepper {
    fn as_fr(&self) -> Fr {
        Fr::from_le_bytes_mod_order(self.to_bytes())
    }
}

pub trait FromB64 {
    fn from_b64(s: &str) -> Result<Self>
    where
        Self: Sized;
}

impl FromB64 for RsaSignature {
    /// JWT signature is encoded in big-endian.
    fn from_b64(s: &str) -> Result<Self> {
        Ok(BigUint::from_bytes_be(&base64::decode_config(
            s,
            base64::URL_SAFE_NO_PAD,
        )?))
    }
}

pub trait FromHex {
    fn from_hex(s: &str) -> Result<Self>
    where
        Self: Sized;
}

impl FromHex for EphemeralPublicKey {
    fn from_hex(s: &str) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(EphemeralPublicKey::try_from(hex::decode(s)?.as_slice())?)
    }
}

#[derive(Debug)]
pub struct JwtParts {
    header: String,
    payload: String,
    signature: String,
}

#[derive(Serialize, Deserialize)]
pub struct JwtHeader {
    pub kid: String,
}

#[derive(Serialize, Deserialize)]
pub struct JwtPayload {
    pub iss: String,
    pub iat: u64,
    pub nonce: String,
    pub sub: Option<String>,
    pub email: Option<String>,
    pub aud: Option<String>,
}

impl FromB64 for JwtParts {
    fn from_b64(s: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let jwt_parts: Vec<&str> = s.split('.').collect();
        Ok(Self {
            header: String::from(
                *jwt_parts
                    .first()
                    .ok_or_else(|| anyhow!("JWT did not parse correctly"))?,
            ),
            payload: String::from(
                *jwt_parts
                    .get(1)
                    .ok_or_else(|| anyhow!("JWT did not parse correctly"))?,
            ),
            signature: String::from(
                *jwt_parts
                    .get(2)
                    .ok_or_else(|| anyhow!("JWT did not parse correctly"))?,
            ),
        })
    }
}

impl JwtParts {
    pub fn unsigned_undecoded(&self) -> String {
        String::from(&self.header) + "." + &self.payload
    }

    pub fn payload_undecoded(&self) -> String {
        String::from(&self.payload)
    }

    pub fn header_undecoded_with_dot(&self) -> String {
        String::from(&self.header) + "."
    }

    pub fn header_decoded(&self) -> Result<String> {
        Ok(String::from_utf8(base64::decode_config(
            &self.header,
            base64::URL_SAFE_NO_PAD,
        )?)?)
    }

    pub fn payload_decoded(&self) -> Result<String> {
        Ok(String::from_utf8(base64::decode_config(
            &self.payload,
            base64::URL_SAFE_NO_PAD,
        )?)?)
    }

    pub fn signature(&self) -> Result<RsaSignature> {
        RsaSignature::from_b64(&self.signature)
    }
}

pub struct UnsignedJwtPartsWithPadding {
    b: Vec<u8>,
}

impl UnsignedJwtPartsWithPadding {
    pub fn from_b64_bytes_with_padding(b: &[u8]) -> Self {
        Self { b: Vec::from(b) }
    }

    pub fn payload_with_padding(&self) -> Result<Vec<u8>> {
        let first_dot = self
            .b
            .iter()
            .position(|c| c == &b'.')
            .ok_or_else(|| anyhow!("Not a valid jwt; has no \".\""))?;

        Ok(Vec::from(&self.b[first_dot + 1..]))
    }
}

/// Trait which signals that this type allows conversion into 64-bit limbs. Used for JWT signature
/// and JWK modulus.
pub trait As64BitLimbs {
    fn as_64bit_limbs(&self) -> Vec<u64>;
}

impl As64BitLimbs for RSA_JWK {
    fn as_64bit_limbs(&self) -> Vec<u64> {
        let modulus_bytes = base64::decode_config(&self.n, base64::URL_SAFE_NO_PAD)
            .expect("JWK should always have a properly-encoded modulus");
        // JWKs encode modulus in big-endian order
        let modulus_biguint: BigUint = BigUint::from_bytes_be(&modulus_bytes);
        modulus_biguint.to_u64_digits()
    }
}

impl As64BitLimbs for RsaSignature {
    fn as_64bit_limbs(&self) -> Vec<u64> {
        self.to_u64_digits()
    }
}

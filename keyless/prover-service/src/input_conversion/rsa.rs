use std::str::FromStr;

use aptos_types::jwks::rsa::RSA_JWK;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rsa;
use rsa::{
    pkcs1::{EncodeRsaPrivateKey, LineEnding},
    traits::PublicKeyParts,
};

use num_bigint::BigUint;


use anyhow::{Error, Result};
const MAX: u64 = 2048;
const BASE: u64 = 64;



#[derive(Debug, PartialEq, Eq)]
pub struct RsaPublicKey {
    modulus: BigUint,
}

impl RsaPublicKey {
    pub fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        Ok(Self {
            modulus: BigUint::from_str(s)?,
        })
    }

    pub fn to_64bit_limbs(&self) -> Vec<u64> {
        self.modulus.to_u64_digits()
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        self.modulus.to_bytes_be()
    }
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            modulus: BigUint::from_bytes_be(bytes),
        }
    }

    // TODO test from and as below
    pub fn from_mod_b64(modulus_b64: &str) -> Result<Self, anyhow::Error> {
        let modulus_bytes = URL_SAFE_NO_PAD.decode(&modulus_b64)?;
        Ok(RsaPublicKey::from_bytes(&modulus_bytes))
    }

    pub fn as_mod_b64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.modulus.to_bytes_be())
    }
}

pub struct RsaPrivateKey {
    internal_private_key: rsa::RsaPrivateKey,
}

impl RsaPrivateKey {

    pub fn new_with_exp<R>(
        rng: &mut R,
        bit_size: usize,
        exp: &BigUint,
    ) -> Result<Self, anyhow::Error>
    where
        R: rsa::rand_core::CryptoRngCore + ?Sized,
    {
        let exp_rsa_type = rsa::BigUint::from_bytes_be(&exp.to_bytes_be());
        Ok(Self {
            internal_private_key: rsa::RsaPrivateKey::new_with_exp(rng, bit_size, &exp_rsa_type)?,
        })
    }

    pub fn as_encoding_key(&self) -> jsonwebtoken::EncodingKey {
        let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(
            self.internal_private_key
            .to_pkcs1_pem(LineEnding::LF)
            .unwrap()
            .as_bytes(),
            )
            .unwrap();
        encoding_key
    }

}

impl From<&RsaPrivateKey> for RsaPublicKey {
    fn from(value: &RsaPrivateKey) -> Self {
        RsaPublicKey {
            modulus: num_bigint::BigUint::from_bytes_be(&value.internal_private_key.n().to_bytes_be())
        }
    }
}


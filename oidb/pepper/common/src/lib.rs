// Copyright © Aptos Foundation

use crate::asymmetric_encryption::{
    AsymmetricEncryption, elgamal_curve25519_aes256_gcm,
    elgamal_curve25519_aes256_gcm::ElGamalCurve25519Aes256Gcm,
};
use aes_gcm::aead::rand_core::{CryptoRng as AeadCryptoRng, RngCore as AeadRngCore};
use anyhow::bail;
use curve25519_dalek::digest::Digest;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;

pub mod asymmetric_encryption;
pub mod elgamal;
pub mod jwt;
pub mod vuf;

pub fn sha3_256(input: &[u8]) -> Vec<u8> {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

/// The spec of a request to this pepper service.
#[derive(Debug, Deserialize, Serialize)]
pub enum PepperRequest {
    V0(PepperRequestV0),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PepperResponse {
    Error(String),
    V0(PepperResponseV0),
}

/// A pepper scheme where:
/// - The pepper input contains `JWT, epk, blinder, expiry_time, uid_key`, wrapped in type `PepperRequestV0`.
/// - The pepper output is the `BLS12381_G1_BLS` VUF output of the input, wrapped in type `PepperResponseV0`.
#[derive(Debug, Deserialize, Serialize)]
pub struct PepperRequestV0 {
    pub jwt: String,
    pub epk_hex_string: String,
    pub epk_expiry_time_secs: u64,
    pub epk_blinder_hex_string: String,
    pub uid_key: Option<String>,
}

/// The response to `PepperRequestV0`, which contains either the pepper or a processing error.
#[derive(Debug, Deserialize, Serialize)]
pub enum PepperResponseV0 {
    Ok(Vec<u8>),
    Err(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VUFVerificationKey {
    pub scheme_name: String,
    pub vuf_public_key_hex_string: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EncryptionPubKey {
    pub scheme_name: String,
    pub payload_hexlified: String,
}

impl EncryptionPubKey {
    /// TODO: adjust the dependencies so they can share a RNG.
    pub fn encrypt<R1: CryptoRng + RngCore, R2: AeadCryptoRng + AeadRngCore>(
        &self,
        main_rng: &mut R1,
        aead_rng: &mut R2,
        msg: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        match self.scheme_name.as_str() {
            // "Scheme0" => {
            //     let pk = hex::decode(self.payload_hexlified.as_bytes())?;
            //     asymmetric_encryption::scheme0::Scheme::enc(rng, pk.as_slice(), msg)
            // }
            elgamal_curve25519_aes256_gcm::SCHEME_NAME => {
                let pk = hex::decode(self.payload_hexlified.as_bytes())?;
                ElGamalCurve25519Aes256Gcm::enc(main_rng, aead_rng, pk.as_slice(), msg)
            },
            _ => bail!("EncryptionPubKey::encrypt failed with unknown scheme"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PepperInput {
    pub iss: String,
    pub aud: String,
    pub uid_val: String,
    pub uid_key: String,
}

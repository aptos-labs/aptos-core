use crate::asymmetric_encryption::AsymmetricEncryption;
use aes_gcm::aead::rand_core::{CryptoRng as AeadCryptoRng, RngCore as AeadRngCore};
use anyhow::bail;
use curve25519_dalek::digest::Digest;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

pub mod asymmetric_encryption;
pub mod elgamal;
pub mod jwt;

pub mod nonce_derivation;
pub mod pepper_pre_image_derivation;
pub mod vuf;

pub fn sha3_256(input: &[u8]) -> Vec<u8> {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

/// The spec of a request to this pepper service.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PepperRequest {
    pub jwt: String,
    /// If specified, generate pepper for `jwk.payload.iss, jwk.payload.sub, overriding_aud`.
    /// Otherwise, generate pepper for `jwk.payload.iss, jwk.payload.sub, jwk.payload.aud`.
    pub overriding_aud: Option<String>,

    pub ephem_pub_key_hexlified: String,
    pub enc_pub_key: EncryptionPubKey,
    pub expiry_time_sec: u64,
    pub blinder_hexlified: String,
    pub uid_key: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SimplePepperRequest {
    pub jwt: String,
    pub uid_key: Option<String>,
}

/// The spec of a response from this pepper service.
#[derive(Debug, Deserialize, Serialize)]
pub enum PepperResponse {
    OK { pepper_encrypted_hexlified: String },
    Error(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum UnencryptedPepperResponse {
    OK { pepper_hexlified: String },
    Error(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VUFVerificationKey {
    pub scheme_name: String,
    pub payload_hexlified: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EncryptionPubKey {
    pub scheme_name: String,
    pub payload_hexlified: String,
}

impl EncryptionPubKey {
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
            "Scheme1" => {
                let pk = hex::decode(self.payload_hexlified.as_bytes())?;
                asymmetric_encryption::scheme1::Scheme::enc(main_rng, aead_rng, pk.as_slice(), msg)
            },
            _ => bail!("EncryptionPubKey::encrypt failed with unknown scheme"),
        }
    }
}

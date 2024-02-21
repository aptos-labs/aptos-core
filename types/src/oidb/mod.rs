// Copyright © Aptos Foundation

use crate::{
    on_chain_config::CurrentTimeMicroseconds,
    transaction::{
        authenticator::{
            AnyPublicKey, AnySignature, EphemeralPublicKey, EphemeralSignature, MAX_NUM_OF_SIGS,
        },
        SignedTransaction,
    },
};
use anyhow::bail;
use aptos_crypto::{poseidon_bn254, CryptoMaterialError, ValidCryptoMaterial};
use ark_bn254::Bn254;
use ark_groth16::PreparedVerifyingKey;
use ark_serialize::CanonicalSerialize;
use base64::URL_SAFE_NO_PAD;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    str,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

mod bn254_circom;
mod circuit_constants;
mod circuit_testcases;
mod configuration;
mod groth16_sig;
mod groth16_vk;
mod openid_sig;
pub mod test_utils;

use crate::oidb::circuit_constants::devnet_prepared_vk;
pub use bn254_circom::get_public_inputs_hash;
pub use configuration::Configuration;
pub use groth16_sig::{Groth16Zkp, SignedGroth16Zkp};
pub use groth16_vk::Groth16VerificationKey;
pub use openid_sig::{Claims, OpenIdSig};

/// The devnet VK that is initialized during genesis.
pub static DEVNET_VERIFICATION_KEY: Lazy<PreparedVerifyingKey<Bn254>> =
    Lazy::new(devnet_prepared_vk);

#[macro_export]
macro_rules! invalid_signature {
    ($message:expr) => {
        VMStatus::error(StatusCode::INVALID_SIGNATURE, Some($message.to_owned()))
    };
}

/// Useful macro for arkworks serialization!
#[macro_export]
macro_rules! serialize {
    ($obj:expr) => {{
        let mut buf = vec![];
        $obj.serialize_compressed(&mut buf).unwrap();
        buf
    }};
}

/// Allows us to support direct verification of OpenID signatures, in the rare case that we would
/// need to turn off ZK proofs due to a bug in the circuit.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub enum ZkpOrOpenIdSig {
    Groth16Zkp(SignedGroth16Zkp),
    OpenIdSig(OpenIdSig),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct OidbSignature {
    /// A \[ZKPoK of an\] OpenID signature over several relevant fields (e.g., `aud`, `sub`, `iss`,
    /// `nonce`) where `nonce` contains a commitment to `ephemeral_pubkey` and an expiration time
    /// `exp_timestamp_secs`.
    pub sig: ZkpOrOpenIdSig,

    /// The base64url-encoded header (no dot at the end), which contains two relevant fields:
    ///  1. `kid`, which indicates which of the OIDC provider's JWKs should be used to verify the
    ///     \[ZKPoK of an\] OpenID signature.,
    ///  2. `alg`, which indicates which type of signature scheme was used to sign the JWT
    pub jwt_header_b64: String,

    /// The expiry time of the `ephemeral_pubkey` represented as a UNIX epoch timestamp in seconds.
    pub exp_timestamp_secs: u64,

    /// A short lived public key used to verify the `ephemeral_signature`.
    pub ephemeral_pubkey: EphemeralPublicKey,
    /// The signature of the transaction signed by the private key of the `ephemeral_pubkey`.
    pub ephemeral_signature: EphemeralSignature,
}

impl TryFrom<&[u8]> for OidbSignature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<OidbSignature>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

impl ValidCryptoMaterial for OidbSignature {
    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JWTHeader {
    pub kid: String,
    pub alg: String,
}

impl OidbSignature {
    /// A reasonable upper bound for the number of bytes we expect in an OIDB public key. This is
    /// enforced by our full nodes when they receive OIDB TXNs.
    pub const MAX_LEN: usize = 4000;

    pub fn parse_jwt_header(&self) -> anyhow::Result<JWTHeader> {
        let jwt_header_json = base64url_decode_as_str(&self.jwt_header_b64)?;
        let header: JWTHeader = serde_json::from_str(&jwt_header_json)?;
        Ok(header)
    }

    pub fn verify_expiry(&self, current_time: &CurrentTimeMicroseconds) -> anyhow::Result<()> {
        let block_time = UNIX_EPOCH + Duration::from_micros(current_time.microseconds);
        let expiry_time = seconds_from_epoch(self.exp_timestamp_secs);

        if block_time > expiry_time {
            bail!("OIDB signature is expired");
        } else {
            Ok(())
        }
    }
}

/// The pepper is used to create a _hiding_ identity commitment (IDC) when deriving an OIDB address.
/// We fix its size at `poseidon_bn254::BYTES_PACKED_PER_SCALAR` to avoid extra hashing work when
/// computing the public inputs hash.
///
/// This value should **NOT* be changed since on-chain addresses are based on it (e.g.,
/// hashing with a larger pepper would lead to a different address).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Pepper(pub(crate) [u8; poseidon_bn254::BYTES_PACKED_PER_SCALAR]);

impl Pepper {
    pub const NUM_BYTES: usize = poseidon_bn254::BYTES_PACKED_PER_SCALAR;

    pub fn new(bytes: [u8; Self::NUM_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(&self) -> &[u8; Self::NUM_BYTES] {
        &self.0
    }

    pub fn from_hex(hex: &str) -> Self {
        let bytes = hex::decode(hex).unwrap();
        let mut extended_bytes = [0u8; Self::NUM_BYTES];
        extended_bytes.copy_from_slice(&bytes);
        Self(extended_bytes)
    }

    // Used for testing. #[cfg(test)] doesn't seem to allow for use in smoke tests.
    pub fn from_number(num: u128) -> Self {
        let big_int = num_bigint::BigUint::from(num);
        let bytes: Vec<u8> = big_int.to_bytes_le();
        let mut extended_bytes = [0u8; Self::NUM_BYTES];
        extended_bytes[..bytes.len()].copy_from_slice(&bytes);
        Self(extended_bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct IdCommitment(#[serde(with = "serde_bytes")] pub(crate) Vec<u8>);

impl IdCommitment {
    /// The max length of the value of the JWT's `aud` field supported in our circuit. OIDB address
    /// derivation depends on this, so it should not be changed.
    pub const MAX_AUD_VAL_BYTES: usize = circuit_constants::MAX_AUD_VAL_BYTES;
    /// The max length of the JWT field name that stores the user's ID (e.g., `sub`, `email`) which is
    /// supported in our circuit. OIDB address derivation depends on this, so it should not be changed.
    pub const MAX_UID_KEY_BYTES: usize = circuit_constants::MAX_UID_KEY_BYTES;
    /// The max length of the value of the JWT's UID field (`sub`, `email`) that stores the user's ID
    /// which is supported in our circuit. OIDB address derivation depends on this, so it should not
    /// be changed.
    pub const MAX_UID_VAL_BYTES: usize = circuit_constants::MAX_UID_VAL_BYTES;
    /// The size of the identity commitment (IDC) used to derive an OIDB address. This value should **NOT*
    /// be changed since on-chain addresses are based on it (e.g., hashing a larger-sized IDC would lead
    /// to a different address).
    pub const NUM_BYTES: usize = 32;

    pub fn new_from_preimage(
        pepper: &Pepper,
        aud: &str,
        uid_key: &str,
        uid_val: &str,
    ) -> anyhow::Result<Self> {
        let aud_val_hash = poseidon_bn254::pad_and_hash_string(aud, Self::MAX_AUD_VAL_BYTES)?;
        // println!("aud_val_hash: {}", aud_val_hash);
        let uid_key_hash = poseidon_bn254::pad_and_hash_string(uid_key, Self::MAX_UID_KEY_BYTES)?;
        // println!("uid_key_hash: {}", uid_key_hash);
        let uid_val_hash = poseidon_bn254::pad_and_hash_string(uid_val, Self::MAX_UID_VAL_BYTES)?;
        // println!("uid_val_hash: {}", uid_val_hash);
        let pepper_scalar = poseidon_bn254::pack_bytes_to_one_scalar(pepper.0.as_slice())?;
        // println!("Pepper Fr: {}", pepper_scalar);

        let fr = poseidon_bn254::hash_scalars(vec![
            pepper_scalar,
            aud_val_hash,
            uid_val_hash,
            uid_key_hash,
        ])?;

        let mut idc_bytes = vec![0u8; IdCommitment::NUM_BYTES];
        fr.serialize_uncompressed(&mut idc_bytes[..])?;
        Ok(IdCommitment(idc_bytes))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

impl TryFrom<&[u8]> for IdCommitment {
    type Error = CryptoMaterialError;

    fn try_from(_value: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<IdCommitment>(_value)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct OidbPublicKey {
    /// The value of the `iss` field from the JWT, indicating the OIDC provider.
    /// e.g., https://accounts.google.com
    pub iss_val: String,

    /// SNARK-friendly commitment to:
    /// 1. The application's ID; i.e., the `aud` field in the signed OIDC JWT representing the OAuth client ID.
    /// 2. The OIDC provider's internal identifier for the user; e.g., the `sub` field in the signed OIDC JWT
    ///    which is Google's internal user identifier for bob@gmail.com, or the `email` field.
    ///
    /// e.g., H(aud || uid_key || uid_val || pepper), where `pepper` is the commitment's randomness used to hide
    ///  `aud` and `sub`.
    pub idc: IdCommitment,
}

impl OidbPublicKey {
    /// A reasonable upper bound for the number of bytes we expect in an OIDB public key. This is
    /// enforced by our full nodes when they receive OIDB TXNs.
    pub const MAX_LEN: usize = 200 + IdCommitment::NUM_BYTES;

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

impl TryFrom<&[u8]> for OidbPublicKey {
    type Error = CryptoMaterialError;

    fn try_from(_value: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<OidbPublicKey>(_value)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

pub fn get_oidb_authenticators(
    transaction: &SignedTransaction,
) -> anyhow::Result<Vec<(OidbPublicKey, OidbSignature)>> {
    // Check all the signers in the TXN
    let single_key_authenticators = transaction
        .authenticator_ref()
        .to_single_key_authenticators()?;
    let mut authenticators = Vec::with_capacity(MAX_NUM_OF_SIGS);
    for authenticator in single_key_authenticators {
        if let (AnyPublicKey::OIDB { public_key }, AnySignature::OIDB { signature }) =
            (authenticator.public_key(), authenticator.signature())
        {
            authenticators.push((public_key.clone(), signature.clone()))
        }
    }
    Ok(authenticators)
}

pub(crate) fn base64url_encode_str(data: &str) -> String {
    base64::encode_config(data.as_bytes(), URL_SAFE_NO_PAD)
}

pub(crate) fn base64url_encode_bytes(data: &[u8]) -> String {
    base64::encode_config(data, URL_SAFE_NO_PAD)
}

fn base64url_decode_as_str(b64: &str) -> anyhow::Result<String> {
    let decoded_bytes = base64::decode_config(b64, URL_SAFE_NO_PAD)?;
    // Convert the decoded bytes to a UTF-8 string
    let str = String::from_utf8(decoded_bytes)?;
    Ok(str)
}

fn seconds_from_epoch(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

#[cfg(test)]
mod tests;

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{
    authenticator::{
        AnyPublicKey, AnySignature, EphemeralPublicKey, EphemeralSignature, MAX_NUM_OF_SIGS,
    },
    SignedTransaction,
};
use anyhow::bail;
use aptos_crypto::{poseidon_bn254, CryptoMaterialError, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use ark_bn254::Bn254;
use ark_groth16::PreparedVerifyingKey;
use ark_serialize::CanonicalSerialize;
use base64::URL_SAFE_NO_PAD;
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    str,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub mod bn254_circom;
pub mod circuit_constants;
pub mod circuit_testcases;
mod configuration;
mod groth16_sig;
mod groth16_vk;
mod openid_sig;
pub mod proof_simulation;
pub mod test_utils;
mod zkp_sig;

use crate::keyless::circuit_constants::prepared_vk_for_testing;
pub use bn254_circom::{
    g1_projective_str_to_affine, g2_projective_str_to_affine, get_public_inputs_hash, G1Bytes,
    G2Bytes, G1_PROJECTIVE_COMPRESSED_NUM_BYTES, G2_PROJECTIVE_COMPRESSED_NUM_BYTES,
};
pub use configuration::Configuration;
pub use groth16_sig::{Groth16Proof, Groth16ProofAndStatement, ZeroKnowledgeSig};
pub use groth16_vk::Groth16VerificationKey;
use move_core_types::account_address::AccountAddress;
pub use openid_sig::{Claims, OpenIdSig};
pub use zkp_sig::ZKP;

/// The name of the Move module for keyless accounts deployed at 0x1.
pub const KEYLESS_ACCOUNT_MODULE_NAME: &str = "keyless_account";

/// A VK that we use often for keyless e2e tests and smoke tests.
pub static VERIFICATION_KEY_FOR_TESTING: Lazy<PreparedVerifyingKey<Bn254>> =
    Lazy::new(prepared_vk_for_testing);

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

/// A signature from the OIDC provider over the user ID, the application ID and the EPK, which serves
/// as a "certificate" binding the EPK to the keyless account associated with that user and application.
///
/// This is a \[ZKPoK of an\] OpenID signature over a JWT containing several relevant fields
/// (e.g., `aud`, `sub`, `iss`, `nonce`) where `nonce` is a commitment to the `ephemeral_pubkey` and
/// the expiration time
/// `exp_timestamp_secs`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum EphemeralCertificate {
    ZeroKnowledgeSig(ZeroKnowledgeSig),
    OpenIdSig(OpenIdSig),
}

/// NOTE: See `KeylessPublicKey` comments for why this cannot be named `Signature`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct KeylessSignature {
    pub cert: EphemeralCertificate,

    /// The decoded/plaintext JWT header (i.e., *not* base64url-encoded), with two relevant fields:
    ///  1. `kid`, which indicates which of the OIDC provider's JWKs should be used to verify the
    ///     \[ZKPoK of an\] OpenID signature.,
    ///  2. `alg`, which indicates which type of signature scheme was used to sign the JWT
    pub jwt_header_json: String,

    /// The expiry time of the `ephemeral_pubkey` represented as a UNIX epoch timestamp in seconds.
    pub exp_date_secs: u64,

    /// A short lived public key used to verify the `ephemeral_signature`.
    pub ephemeral_pubkey: EphemeralPublicKey,

    /// A signature over the transaction and, if present, the ZKP, under `ephemeral_pubkey`.
    /// The ZKP is included in this signature to prevent malleability attacks.
    pub ephemeral_signature: EphemeralSignature,
}

/// This struct wraps the transaction and optional ZKP that is signed with the ephemeral secret key.
#[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct TransactionAndProof<T> {
    pub message: T,
    pub proof: Option<ZKP>,
}

impl TryFrom<&[u8]> for KeylessSignature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<KeylessSignature>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

impl ValidCryptoMaterial for KeylessSignature {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JWTHeader {
    pub kid: String,
    pub alg: String,
}

impl KeylessSignature {
    /// A reasonable upper bound for the number of bytes we expect in a keyless signature. This is
    /// enforced by our full nodes when they receive TXNs.
    pub const MAX_LEN: usize = 4000;

    pub fn parse_jwt_header(&self) -> anyhow::Result<JWTHeader> {
        let header: JWTHeader = serde_json::from_str(&self.jwt_header_json)?;
        Ok(header)
    }

    pub fn verify_expiry(&self, current_time_microseconds: u64) -> anyhow::Result<()> {
        let block_time = UNIX_EPOCH.checked_add(Duration::from_micros(current_time_microseconds))
            .ok_or_else(|| anyhow::anyhow!("Overflowed on UNIX_EPOCH + current_time_microseconds when checking exp_date_secs"))?;
        let expiry_time = seconds_from_epoch(self.exp_date_secs)?;

        if block_time > expiry_time {
            bail!("Keyless signature is expired");
        } else {
            Ok(())
        }
    }
}

/// The pepper is used to create a _hiding_ identity commitment (IDC) when deriving a keyless address.
/// We fix its size at `poseidon_bn254::keyless::BYTES_PACKED_PER_SCALAR` to avoid extra hashing work when
/// computing the public inputs hash.
///
/// This value should **NOT* be changed since on-chain addresses are based on it (e.g.,
/// hashing with a larger pepper would lead to a different address).
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct Pepper(pub(crate) [u8; poseidon_bn254::keyless::BYTES_PACKED_PER_SCALAR]);

impl Pepper {
    pub const NUM_BYTES: usize = poseidon_bn254::keyless::BYTES_PACKED_PER_SCALAR;

    pub fn new(bytes: [u8; Self::NUM_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(&self) -> &[u8; Self::NUM_BYTES] {
        &self.0
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

impl<'de> Deserialize<'de> for Pepper {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = <String>::deserialize(deserializer)?;
            let bytes = hex::decode(s)
                .map_err(serde::de::Error::custom)?
                .try_into()
                .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?;

            Ok(Pepper::new(bytes))
        } else {
            // In order to preserve the Serde data model and help analysis tools,
            // make sure to wrap our value in a container with the same name
            // as the original type.
            #[derive(::serde::Deserialize)]
            #[serde(rename = "Pepper")]
            struct Value([u8; Pepper::NUM_BYTES]);

            let value = Value::deserialize(deserializer)?;
            Ok(Pepper::new(value.0))
        }
    }
}

impl Serialize for Pepper {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            hex::encode(self.0).serialize(serializer)
        } else {
            // See comment in deserialize.
            serializer.serialize_newtype_struct("Pepper", &self.0)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct IdCommitment(#[serde(with = "serde_bytes")] pub(crate) Vec<u8>);

impl IdCommitment {
    /// The max length of the value of the JWT's `aud` field supported in our circuit. Keyless address
    /// derivation depends on this, so it should not be changed.
    pub const MAX_AUD_VAL_BYTES: usize = circuit_constants::MAX_AUD_VAL_BYTES;
    /// The max length of the JWT field name that stores the user's ID (e.g., `sub`, `email`) which is
    /// supported in our circuit. Keyless address derivation depends on this, so it should not be changed.
    pub const MAX_UID_KEY_BYTES: usize = circuit_constants::MAX_UID_KEY_BYTES;
    /// The max length of the value of the JWT's UID field (`sub`, `email`) that stores the user's ID
    /// which is supported in our circuit. Keyless address derivation depends on this, so it should not
    /// be changed.
    pub const MAX_UID_VAL_BYTES: usize = circuit_constants::MAX_UID_VAL_BYTES;
    /// The size of the identity commitment (IDC) used to derive a keyless address. This value should **NOT*
    /// be changed since on-chain addresses are based on it (e.g., hashing a larger-sized IDC would lead
    /// to a different address).
    pub const NUM_BYTES: usize = 32;

    pub fn new_from_preimage(
        pepper: &Pepper,
        aud: &str,
        uid_key: &str,
        uid_val: &str,
    ) -> anyhow::Result<Self> {
        let aud_val_hash =
            poseidon_bn254::keyless::pad_and_hash_string(aud, Self::MAX_AUD_VAL_BYTES)?;
        // println!("aud_val_hash: {}", aud_val_hash);
        let uid_key_hash =
            poseidon_bn254::keyless::pad_and_hash_string(uid_key, Self::MAX_UID_KEY_BYTES)?;
        // println!("uid_key_hash: {}", uid_key_hash);
        let uid_val_hash =
            poseidon_bn254::keyless::pad_and_hash_string(uid_val, Self::MAX_UID_VAL_BYTES)?;
        // println!("uid_val_hash: {}", uid_val_hash);
        let pepper_scalar = poseidon_bn254::keyless::pack_bytes_to_one_scalar(pepper.0.as_slice())?;
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

/// NOTE: Could not use keyless::PublicKey here due to the way `testsuite/generate-format` works.
/// Would need to use `#[key_name(<some_other_name>)]` to avoid naming conflicts with another
/// `PublicKey` struct. But the `key_name` procedural macro only works with the `[De]SerializeKey`
/// procedural macros, which we cannot use since they force us to reimplement serialization.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct KeylessPublicKey {
    /// The value of the `iss` field from the JWT, indicating the OIDC provider.
    /// e.g., <https://accounts.google.com>
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

/// Unlike a normal keyless account, a "federated" keyless account will accept JWKs published at a
/// specific contract address.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct FederatedKeylessPublicKey {
    pub jwk_addr: AccountAddress,
    pub pk: KeylessPublicKey,
}

impl FederatedKeylessPublicKey {
    /// A reasonable upper bound for the number of bytes we expect in a federated keyless public key.
    /// This is enforced by our full nodes when they receive TXNs.
    pub const MAX_LEN: usize = AccountAddress::LENGTH + KeylessPublicKey::MAX_LEN;

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

impl TryFrom<&[u8]> for FederatedKeylessPublicKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<FederatedKeylessPublicKey>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum AnyKeylessPublicKey {
    Normal(KeylessPublicKey),
    Federated(FederatedKeylessPublicKey),
}

impl AnyKeylessPublicKey {
    pub fn inner_keyless_pk(&self) -> &KeylessPublicKey {
        match self {
            AnyKeylessPublicKey::Normal(pk) => pk,
            AnyKeylessPublicKey::Federated(fed_pk) => &fed_pk.pk,
        }
    }
}

impl From<AnyKeylessPublicKey> for AnyPublicKey {
    fn from(apk: AnyKeylessPublicKey) -> Self {
        match apk {
            AnyKeylessPublicKey::Normal(pk) => AnyPublicKey::Keyless { public_key: pk },
            AnyKeylessPublicKey::Federated(fed_pk) => {
                AnyPublicKey::FederatedKeyless { public_key: fed_pk }
            },
        }
    }
}

impl KeylessPublicKey {
    /// A reasonable upper bound for the number of bytes we expect in a keyless public key. This is
    /// enforced by our full nodes when they receive TXNs.
    pub const MAX_LEN: usize = 200 + IdCommitment::NUM_BYTES;

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

impl TryFrom<&[u8]> for KeylessPublicKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<KeylessPublicKey>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

pub fn get_authenticators(
    transaction: &SignedTransaction,
) -> anyhow::Result<Vec<(AnyKeylessPublicKey, KeylessSignature)>> {
    // Check all the signers in the TXN
    let single_key_authenticators = transaction
        .authenticator_ref()
        .to_single_key_authenticators()?;
    let mut authenticators = Vec::with_capacity(MAX_NUM_OF_SIGS);
    for authenticator in single_key_authenticators {
        match (authenticator.public_key(), authenticator.signature()) {
            (AnyPublicKey::Keyless { public_key }, AnySignature::Keyless { signature }) => {
                authenticators.push((
                    AnyKeylessPublicKey::Normal(public_key.clone()),
                    signature.clone(),
                ))
            },
            (
                AnyPublicKey::FederatedKeyless { public_key },
                AnySignature::Keyless { signature },
            ) => authenticators.push((
                AnyKeylessPublicKey::Federated(FederatedKeylessPublicKey {
                    jwk_addr: public_key.jwk_addr,
                    pk: public_key.pk.clone(),
                }),
                signature.clone(),
            )),
            _ => {
                // ignore.
            },
        }
    }
    Ok(authenticators)
}

pub fn base64url_encode_str(data: &str) -> String {
    base64::encode_config(data.as_bytes(), URL_SAFE_NO_PAD)
}

pub(crate) fn base64url_encode_bytes(data: &[u8]) -> String {
    base64::encode_config(data, URL_SAFE_NO_PAD)
}

#[allow(unused)]
fn base64url_decode_as_str(b64: &str) -> anyhow::Result<String> {
    let decoded_bytes = base64::decode_config(b64, URL_SAFE_NO_PAD)?;
    // Convert the decoded bytes to a UTF-8 string
    let str = String::from_utf8(decoded_bytes)?;
    Ok(str)
}

fn seconds_from_epoch(secs: u64) -> anyhow::Result<SystemTime> {
    UNIX_EPOCH
        .checked_add(Duration::from_secs(secs))
        .ok_or_else(|| anyhow::anyhow!("Overflowed on UNIX_EPOCH + secs in seconds_from_epoch"))
}

#[cfg(test)]
mod tests;

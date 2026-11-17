// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! ML-DSA-65 (FIPS 204) consensus signatures. EXPERIMENT ONLY.
//!
//! The ML-DSA key pair of a validator is derived from its BLS public key, which is public.
//! Anyone can derive the signing key of any validator, so these signatures give no
//! security. This keeps genesis, the staking module, and key storage unchanged while the
//! signing cost, verification cost, and message sizes are the real ML-DSA-65 ones.
//!
//! ML-DSA has no aggregation. A certificate carries one signature per signer.

use aptos_crypto::{
    bls12381,
    hash::{CryptoHash, HashValue},
    traits::signing_message,
    CryptoMaterialError,
};
use libcrux_ml_dsa::ml_dsa_65::{
    self, MLDSA65Signature, MLDSA65SigningKey, MLDSA65VerificationKey,
};
use serde::{
    de::{self, Deserializer},
    ser::Serializer,
    Deserialize, Serialize,
};
use std::{fmt, sync::Arc};

const KEYGEN_DOMAIN: &[u8] = b"APTOS_PQ_CONSENSUS_EXPERIMENT_INSECURE_KEYGEN";
const SIGNING_CONTEXT: &[u8] = b"APTOS_CONSENSUS";
const SIGNATURE_NAME: &str = "MlDsa65Signature";

fn keygen_seed(bls_public_key: &bls12381::PublicKey) -> [u8; 32] {
    let mut input = KEYGEN_DOMAIN.to_vec();
    input.extend_from_slice(&bls_public_key.to_bytes());
    let mut seed = [0u8; 32];
    seed.copy_from_slice(HashValue::sha3_256_of(&input).as_ref());
    seed
}

pub struct MlDsaSigningKey(MLDSA65SigningKey);

impl MlDsaSigningKey {
    pub fn derive(bls_public_key: &bls12381::PublicKey) -> Self {
        Self(ml_dsa_65::generate_key_pair(keygen_seed(bls_public_key)).signing_key)
    }

    pub fn sign<T: Serialize + CryptoHash>(
        &self,
        message: &T,
    ) -> Result<MlDsaSignature, CryptoMaterialError> {
        let bytes = signing_message(message)?;
        // Deterministic signing. Like BLS, one key gives one signature for one message.
        ml_dsa_65::sign(&self.0, &bytes, SIGNING_CONTEXT, [0u8; 32])
            .map(|signature| MlDsaSignature(Arc::new(signature)))
            .map_err(|_| CryptoMaterialError::ValidationError)
    }
}

impl fmt::Debug for MlDsaSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MlDsaSigningKey(<elided>)")
    }
}

pub struct MlDsaVerificationKey(MLDSA65VerificationKey);

impl MlDsaVerificationKey {
    pub fn derive(bls_public_key: &bls12381::PublicKey) -> Self {
        Self(ml_dsa_65::generate_key_pair(keygen_seed(bls_public_key)).verification_key)
    }

    pub fn verify<T: Serialize + CryptoHash>(
        &self,
        message: &T,
        signature: &MlDsaSignature,
    ) -> Result<(), CryptoMaterialError> {
        self.verify_signing_message(&signing_message(message)?, signature)
    }

    /// Verifies a signature on the output of `signing_message`. Use this to verify many
    /// signatures on one message without serializing the message again.
    pub fn verify_signing_message(
        &self,
        message_bytes: &[u8],
        signature: &MlDsaSignature,
    ) -> Result<(), CryptoMaterialError> {
        ml_dsa_65::verify(&self.0, message_bytes, SIGNING_CONTEXT, &signature.0)
            .map_err(|_| CryptoMaterialError::ValidationError)
    }
}

impl fmt::Debug for MlDsaVerificationKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MlDsaVerificationKey(<elided>)")
    }
}

/// The ML-DSA-65 signature of one validator. A clone shares the signature bytes.
#[derive(Clone)]
pub struct MlDsaSignature(Arc<MLDSA65Signature>);

impl MlDsaSignature {
    pub const LENGTH: usize = 3309;

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.as_slice().to_vec()
    }

    fn from_array(bytes: [u8; Self::LENGTH]) -> Self {
        Self(Arc::new(MLDSA65Signature::new(bytes)))
    }

    /// Mirrors `LazyBlsSignature::from_signature`. ML-DSA needs no lazy form.
    pub fn from_signature(signature: &Self) -> Self {
        signature.clone()
    }

    /// Mirrors `LazyBlsSignature::decompress`. ML-DSA needs no lazy form.
    pub fn decompress(&self) -> Result<Self, CryptoMaterialError> {
        Ok(self.clone())
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_signature() -> Self {
        Self::from_array([0u8; Self::LENGTH])
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn from_raw_bytes_for_test(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_array(bytes)
    }
}

impl TryFrom<&[u8]> for MlDsaSignature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        <[u8; Self::LENGTH]>::try_from(bytes)
            .map(Self::from_array)
            .map_err(|_| CryptoMaterialError::WrongLengthError)
    }
}

impl PartialEq for MlDsaSignature {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_slice() == other.0.as_slice()
    }
}

impl Eq for MlDsaSignature {}

impl std::hash::Hash for MlDsaSignature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_slice().hash(state);
    }
}

// Shows a short prefix only. A full signature is 3309 bytes.
impl fmt::Display for MlDsaSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{}..", hex::encode(&self.0.as_slice()[..8]))
    }
}

impl fmt::Debug for MlDsaSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MlDsaSignature({})", self)
    }
}

impl Serialize for MlDsaSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&format!("0x{}", hex::encode(self.0.as_slice())))
        } else {
            serializer.serialize_newtype_struct(
                SIGNATURE_NAME,
                serde_bytes::Bytes::new(self.0.as_slice()),
            )
        }
    }
}

impl<'de> Deserialize<'de> for MlDsaSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        fn from_slice<E: de::Error>(bytes: &[u8]) -> Result<MlDsaSignature, E> {
            MlDsaSignature::try_from(bytes).map_err(|_| {
                E::custom(format!(
                    "invalid ML-DSA-65 signature length: {} (expected {})",
                    bytes.len(),
                    MlDsaSignature::LENGTH
                ))
            })
        }

        if deserializer.is_human_readable() {
            let encoded = <String>::deserialize(deserializer)?;
            let stripped = encoded.strip_prefix("0x").unwrap_or(&encoded);
            from_slice(&hex::decode(stripped).map_err(de::Error::custom)?)
        } else {
            #[derive(Deserialize)]
            #[serde(rename = "MlDsa65Signature")]
            struct Value<'a>(&'a [u8]);

            from_slice(Value::deserialize(deserializer)?.0)
        }
    }
}

/// The signatures of a set of validators on a certificate, in ascending order of
/// validator index. This is the ML-DSA replacement for a BLS aggregate signature.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MlDsaMultiSignature(Vec<MlDsaSignature>);

impl MlDsaMultiSignature {
    pub fn new(signatures: Vec<MlDsaSignature>) -> Self {
        Self(signatures)
    }

    pub fn signatures(&self) -> &[MlDsaSignature] {
        &self.0
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0
            .iter()
            .flat_map(|signature| signature.0.as_slice().iter().copied())
            .collect()
    }

    /// Mirrors `LazyBlsSignature::from_signature`. ML-DSA needs no lazy form.
    pub fn from_signature(signature: &Self) -> Self {
        signature.clone()
    }

    /// Mirrors `LazyBlsSignature::decompress`. ML-DSA needs no lazy form.
    pub fn decompress(&self) -> Result<Self, CryptoMaterialError> {
        Ok(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{test_utils::TestAptosCrypto, PrivateKey, Uniform};

    fn key_pair() -> (MlDsaSigningKey, MlDsaVerificationKey) {
        let bls_public_key = bls12381::PrivateKey::generate(&mut rand::thread_rng()).public_key();
        (
            MlDsaSigningKey::derive(&bls_public_key),
            MlDsaVerificationKey::derive(&bls_public_key),
        )
    }

    #[test]
    fn sign_and_verify() {
        let (signing_key, verification_key) = key_pair();
        let message = TestAptosCrypto("hello".to_string());
        let signature = signing_key.sign(&message).unwrap();
        assert_eq!(signature.to_bytes().len(), MlDsaSignature::LENGTH);
        verification_key.verify(&message, &signature).unwrap();

        let other_message = TestAptosCrypto("goodbye".to_string());
        assert!(verification_key.verify(&other_message, &signature).is_err());

        let (_, other_verification_key) = key_pair();
        assert!(other_verification_key.verify(&message, &signature).is_err());
    }

    #[test]
    fn serde_round_trip() {
        let (signing_key, _) = key_pair();
        let signature = signing_key
            .sign(&TestAptosCrypto("hello".to_string()))
            .unwrap();

        let bcs_bytes = bcs::to_bytes(&signature).unwrap();
        // ULEB128 length prefix of 3309 is two bytes.
        assert_eq!(bcs_bytes.len(), MlDsaSignature::LENGTH + 2);
        assert_eq!(
            bcs::from_bytes::<MlDsaSignature>(&bcs_bytes).unwrap(),
            signature
        );

        let json = serde_json::to_string(&signature).unwrap();
        assert_eq!(
            serde_json::from_str::<MlDsaSignature>(&json).unwrap(),
            signature
        );

        let multi_signature = MlDsaMultiSignature::new(vec![signature.clone(), signature]);
        let bcs_bytes = bcs::to_bytes(&multi_signature).unwrap();
        assert_eq!(
            bcs::from_bytes::<MlDsaMultiSignature>(&bcs_bytes).unwrap(),
            multi_signature
        );
    }

    #[test]
    fn rejects_wrong_length() {
        let short = serde_bytes::ByteBuf::from(vec![0u8; MlDsaSignature::LENGTH - 1]);
        let bytes = bcs::to_bytes(&short).unwrap();
        assert!(bcs::from_bytes::<MlDsaSignature>(&bytes).is_err());
    }
}

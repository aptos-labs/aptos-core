// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! FN-DSA-512 (Falcon-512) consensus signatures. EXPERIMENT ONLY.
//!
//! The Falcon key pair of a validator is derived from its BLS public key, which is public.
//! Anyone can derive the signing key of any validator, so these signatures give no
//! security. This keeps genesis, the staking module, and key storage unchanged while the
//! signing cost, verification cost, and message sizes are the real FN-DSA-512 ones.
//!
//! Falcon has no aggregation. A certificate carries one signature per signer.

use aptos_crypto::{
    bls12381,
    hash::{CryptoHash, HashValue},
    traits::signing_message,
    CryptoMaterialError,
};
use aptos_fn_dsa::{
    keygen_from_seed, verify, ExpandedSigningKey, SIGNATURE_LENGTH, SIGN_SEED_LENGTH,
    VERIFYING_KEY_LENGTH,
};
use serde::{
    de::{self, Deserializer},
    ser::Serializer,
    Deserialize, Serialize,
};
use std::{fmt, sync::Arc};

const KEYGEN_DOMAIN: &[u8] = b"APTOS_PQ_CONSENSUS_FALCON_EXPERIMENT_INSECURE_KEYGEN";
const SIGN_SEED_DOMAIN: &[u8] = b"APTOS_PQ_CONSENSUS_FALCON_SIGN_SEED";
const SIGNING_CONTEXT: &[u8] = b"APTOS_CONSENSUS";
const SIGNATURE_NAME: &str = "Falcon512Signature";

fn keygen_seed(bls_public_key: &bls12381::PublicKey) -> [u8; 32] {
    let mut input = KEYGEN_DOMAIN.to_vec();
    input.extend_from_slice(&bls_public_key.to_bytes());
    *HashValue::sha3_256_of(&input)
}

pub struct FalconSigningKey {
    key: ExpandedSigningKey,
    /// Secret input for the per-message signing seeds, so signing is deterministic.
    seed_key: [u8; 32],
}

impl FalconSigningKey {
    pub fn derive(bls_public_key: &bls12381::PublicKey) -> Self {
        let seed = keygen_seed(bls_public_key);
        let key_pair = keygen_from_seed(&seed);
        let key = ExpandedSigningKey::new(key_pair.signing_key.as_slice())
            .expect("a freshly generated FN-DSA signing key must decode");
        let mut seed_key_input = SIGN_SEED_DOMAIN.to_vec();
        seed_key_input.extend_from_slice(&seed);
        Self {
            key,
            seed_key: *HashValue::sha3_256_of(&seed_key_input),
        }
    }

    pub fn sign<T: Serialize + CryptoHash>(
        &self,
        message: &T,
    ) -> Result<FalconSignature, CryptoMaterialError> {
        let bytes = signing_message(message)?;
        // Deterministic signing. Like BLS, one key gives one signature for one message.
        let signature = self
            .key
            .sign(SIGNING_CONTEXT, &bytes, &self.sign_seed(&bytes));
        Ok(FalconSignature(Arc::new(signature)))
    }

    fn sign_seed(&self, message_bytes: &[u8]) -> [u8; SIGN_SEED_LENGTH] {
        let mut seed = [0u8; SIGN_SEED_LENGTH];
        for (index, chunk) in seed.chunks_mut(HashValue::LENGTH).enumerate() {
            let mut input = self.seed_key.to_vec();
            input.push(index as u8);
            input.extend_from_slice(message_bytes);
            chunk.copy_from_slice(&HashValue::sha3_256_of(&input).as_ref()[..chunk.len()]);
        }
        seed
    }
}

impl fmt::Debug for FalconSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FalconSigningKey(<elided>)")
    }
}

pub struct FalconVerificationKey([u8; VERIFYING_KEY_LENGTH]);

impl FalconVerificationKey {
    /// Takes milliseconds (FN-DSA key generation). Use
    /// `consensus_signature::pq_verification_key`, which caches the keys.
    pub fn derive(bls_public_key: &bls12381::PublicKey) -> Self {
        Self(keygen_from_seed(&keygen_seed(bls_public_key)).verifying_key)
    }

    pub fn verify<T: Serialize + CryptoHash>(
        &self,
        message: &T,
        signature: &FalconSignature,
    ) -> Result<(), CryptoMaterialError> {
        self.verify_signing_message(&signing_message(message)?, signature)
    }

    /// Verifies a signature on the output of `signing_message`. Use this to verify many
    /// signatures on one message without serializing the message again.
    pub fn verify_signing_message(
        &self,
        message_bytes: &[u8],
        signature: &FalconSignature,
    ) -> Result<(), CryptoMaterialError> {
        if verify(
            signature.0.as_slice(),
            &self.0,
            SIGNING_CONTEXT,
            message_bytes,
        ) {
            Ok(())
        } else {
            Err(CryptoMaterialError::ValidationError)
        }
    }
}

impl fmt::Debug for FalconVerificationKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FalconVerificationKey(<elided>)")
    }
}

/// The FN-DSA-512 signature of one validator. A clone shares the signature bytes.
#[derive(Clone)]
pub struct FalconSignature(Arc<[u8; SIGNATURE_LENGTH]>);

impl FalconSignature {
    pub const LENGTH: usize = SIGNATURE_LENGTH;

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    fn from_array(bytes: [u8; Self::LENGTH]) -> Self {
        Self(Arc::new(bytes))
    }

    /// Mirrors `LazyBlsSignature::from_signature`. Falcon needs no lazy form.
    pub fn from_signature(signature: &Self) -> Self {
        signature.clone()
    }

    /// Mirrors `LazyBlsSignature::decompress`. Falcon needs no lazy form.
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

impl TryFrom<&[u8]> for FalconSignature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        <[u8; Self::LENGTH]>::try_from(bytes)
            .map(Self::from_array)
            .map_err(|_| CryptoMaterialError::WrongLengthError)
    }
}

impl PartialEq for FalconSignature {
    fn eq(&self, other: &Self) -> bool {
        self.0[..] == other.0[..]
    }
}

impl Eq for FalconSignature {}

impl std::hash::Hash for FalconSignature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0[..].hash(state);
    }
}

// Shows a short prefix only. A full signature is 666 bytes.
impl fmt::Display for FalconSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{}..", hex::encode(&self.0[..8]))
    }
}

impl fmt::Debug for FalconSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FalconSignature({})", self)
    }
}

impl Serialize for FalconSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&format!("0x{}", hex::encode(&self.0[..])))
        } else {
            serializer
                .serialize_newtype_struct(SIGNATURE_NAME, serde_bytes::Bytes::new(&self.0[..]))
        }
    }
}

impl<'de> Deserialize<'de> for FalconSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        fn from_slice<E: de::Error>(bytes: &[u8]) -> Result<FalconSignature, E> {
            FalconSignature::try_from(bytes).map_err(|_| {
                E::custom(format!(
                    "invalid FN-DSA-512 signature length: {} (expected {})",
                    bytes.len(),
                    FalconSignature::LENGTH
                ))
            })
        }

        if deserializer.is_human_readable() {
            let encoded = <String>::deserialize(deserializer)?;
            let stripped = encoded.strip_prefix("0x").unwrap_or(&encoded);
            from_slice(&hex::decode(stripped).map_err(de::Error::custom)?)
        } else {
            #[derive(Deserialize)]
            #[serde(rename = "Falcon512Signature")]
            struct Value<'a>(&'a [u8]);

            from_slice(Value::deserialize(deserializer)?.0)
        }
    }
}

/// The signatures of a set of validators on a certificate, in ascending order of
/// validator index. This is the Falcon replacement for a BLS aggregate signature.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FalconMultiSignature(Vec<FalconSignature>);

impl FalconMultiSignature {
    pub fn new(signatures: Vec<FalconSignature>) -> Self {
        Self(signatures)
    }

    pub fn signatures(&self) -> &[FalconSignature] {
        &self.0
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0
            .iter()
            .flat_map(|signature| signature.0.iter().copied())
            .collect()
    }

    /// Mirrors `LazyBlsSignature::from_signature`. Falcon needs no lazy form.
    pub fn from_signature(signature: &Self) -> Self {
        signature.clone()
    }

    /// Mirrors `LazyBlsSignature::decompress`. Falcon needs no lazy form.
    pub fn decompress(&self) -> Result<Self, CryptoMaterialError> {
        Ok(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{test_utils::TestAptosCrypto, PrivateKey, Uniform};

    fn bls_public_key() -> bls12381::PublicKey {
        bls12381::PrivateKey::generate(&mut rand::thread_rng()).public_key()
    }

    #[test]
    fn sign_and_verify() {
        let bls_public_key = bls_public_key();
        let signing_key = FalconSigningKey::derive(&bls_public_key);
        let verification_key = FalconVerificationKey::derive(&bls_public_key);
        let message = TestAptosCrypto("hello".to_string());
        let signature = signing_key.sign(&message).unwrap();
        assert_eq!(signature.to_bytes().len(), FalconSignature::LENGTH);
        verification_key.verify(&message, &signature).unwrap();
        // Deterministic signing.
        assert_eq!(signing_key.sign(&message).unwrap(), signature);

        let other_message = TestAptosCrypto("goodbye".to_string());
        assert!(verification_key.verify(&other_message, &signature).is_err());
        let other_key = FalconVerificationKey::derive(&self::bls_public_key());
        assert!(other_key.verify(&message, &signature).is_err());
    }

    #[test]
    fn serde_round_trip() {
        let signing_key = FalconSigningKey::derive(&bls_public_key());
        let signature = signing_key
            .sign(&TestAptosCrypto("hello".to_string()))
            .unwrap();
        let bcs_bytes = bcs::to_bytes(&signature).unwrap();
        // ULEB128 length prefix of 666 is two bytes.
        assert_eq!(bcs_bytes.len(), FalconSignature::LENGTH + 2);
        assert_eq!(
            bcs::from_bytes::<FalconSignature>(&bcs_bytes).unwrap(),
            signature
        );
        let json = serde_json::to_string(&signature).unwrap();
        assert_eq!(
            serde_json::from_str::<FalconSignature>(&json).unwrap(),
            signature
        );

        let multi_signature = FalconMultiSignature::new(vec![signature.clone(), signature]);
        let bcs_bytes = bcs::to_bytes(&multi_signature).unwrap();
        assert_eq!(
            bcs::from_bytes::<FalconMultiSignature>(&bcs_bytes).unwrap(),
            multi_signature
        );
    }

    #[test]
    fn rejects_wrong_length() {
        let short = serde_bytes::ByteBuf::from(vec![0u8; FalconSignature::LENGTH - 1]);
        let bytes = bcs::to_bytes(&short).unwrap();
        assert!(bcs::from_bytes::<FalconSignature>(&bytes).is_err());
    }
}

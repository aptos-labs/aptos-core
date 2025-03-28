// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides APIs for private keys and public keys used in Secp256k1 ecdsa.

use crate::{
    hash::{CryptoHash, HashValue},
    traits,
    traits::{CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt},
};
use anyhow::{anyhow, Result};
use aptos_crypto_derive::{key_name, DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use core::convert::TryFrom;
use serde::Serialize;

/// libsecp256k1 expects pre-hashed messages of 32-bytes.
pub const MESSAGE_LENGTH: usize = 32;
/// Secp256k1 ecdsa private keys are 256-bit.
pub const PRIVATE_KEY_LENGTH: usize = 32;
/// Secp256k1 ecdsa public keys contain a prefix indicating compression and two 32-byte coordinates.
pub const PUBLIC_KEY_LENGTH: usize = 65;
/// Secp256k1 ecdsa signatures are 256-bit.
pub const SIGNATURE_LENGTH: usize = 64;

/// Secp256k1 ecdsa private key
#[derive(DeserializeKey, Eq, PartialEq, SerializeKey, SilentDebug, SilentDisplay)]
#[key_name("Secp256k1EcdsaPrivateKey")]
pub struct PrivateKey(pub(crate) libsecp256k1::SecretKey);

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(PrivateKey: Clone);

#[cfg(any(test, feature = "cloneable-private-keys"))]
impl Clone for PrivateKey {
    fn clone(&self) -> Self {
        let serialized: &[u8] = &(self.to_bytes());
        PrivateKey::try_from(serialized).unwrap()
    }
}

impl PrivateKey {
    /// Serialize the private key into a byte vector
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.serialize().to_vec()
    }

    fn sign(&self, message: &libsecp256k1::Message) -> Signature {
        let (signature, _recovery_id) = libsecp256k1::sign(message, &self.0);
        Signature(signature)
    }

    /// Private function aimed at minimizing code duplication between sign
    /// methods of the SigningKey implementation. This should remain private.
    #[cfg(any(test, feature = "fuzzing"))]
    fn sign_arbitrary_message(&self, message: &[u8]) -> Signature {
        let message =
            bytes_to_message(message).expect("Consistently hashed to 32-bytes, should never fail.");
        // libsecp256k1 ensures that the s in signature is normalized
        self.sign(&message)
    }
}

impl TryFrom<&[u8]> for PrivateKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<PrivateKey, CryptoMaterialError> {
        match libsecp256k1::SecretKey::parse_slice(bytes) {
            Ok(private_key) => Ok(PrivateKey(private_key)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }
}

impl traits::Length for PrivateKey {
    fn length(&self) -> usize {
        PRIVATE_KEY_LENGTH
    }
}

impl traits::PrivateKey for PrivateKey {
    type PublicKeyMaterial = PublicKey;
}

impl traits::SigningKey for PrivateKey {
    type SignatureMaterial = Signature;
    type VerifyingKeyMaterial = PublicKey;

    fn sign<T: CryptoHash + Serialize>(
        &self,
        message: &T,
    ) -> Result<Signature, CryptoMaterialError> {
        match bytes_to_message(&traits::signing_message(message)?) {
            Ok(message) => Ok(self.sign(&message)),
            Err(_) => Err(CryptoMaterialError::SerializationError),
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn sign_arbitrary_message(&self, message: &[u8]) -> Signature {
        PrivateKey::sign_arbitrary_message(self, message)
    }
}

impl traits::Uniform for PrivateKey {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore,
    {
        loop {
            let mut ret = [0u8; PRIVATE_KEY_LENGTH];
            rng.fill_bytes(&mut ret);
            if let Ok(key) = libsecp256k1::SecretKey::parse(&ret) {
                return Self(key);
            }
        }
    }
}

impl ValidCryptoMaterial for PrivateKey {
    const AIP_80_PREFIX: &'static str = "secp256k1-priv-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

/// Secp256k1 ecds public key
#[derive(DeserializeKey, Clone, Eq, PartialEq, SerializeKey)]
#[key_name("Secp256k1EcdsaPublicKey")]
pub struct PublicKey(pub(crate) libsecp256k1::PublicKey);

impl PublicKey {
    /// Serialize the public key into a byte vector (full length)
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.serialize().to_vec()
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "secp256k1_ecdsa::PublicKey({})", self)
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.to_bytes()[..]))
    }
}

impl std::hash::Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_public_key = self.to_bytes();
        state.write(&encoded_public_key);
    }
}

impl TryFrom<&[u8]> for PublicKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<PublicKey, CryptoMaterialError> {
        match libsecp256k1::PublicKey::parse_slice(bytes, None) {
            Ok(public_key) => Ok(PublicKey(public_key)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }
}

impl From<&PrivateKey> for PublicKey {
    fn from(private_key: &PrivateKey) -> Self {
        PublicKey(libsecp256k1::PublicKey::from_secret_key(&private_key.0))
    }
}

impl traits::PublicKey for PublicKey {
    type PrivateKeyMaterial = PrivateKey;
}

impl traits::Length for PublicKey {
    fn length(&self) -> usize {
        PUBLIC_KEY_LENGTH
    }
}

impl ValidCryptoMaterial for PublicKey {
    const AIP_80_PREFIX: &'static str = "secp256k1-pub-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl traits::VerifyingKey for PublicKey {
    type SignatureMaterial = Signature;
    type SigningKeyMaterial = PrivateKey;
}

/// Secp256k1 ecdsa signature
#[derive(DeserializeKey, Clone, SerializeKey)]
#[key_name("Secp256k1EcdsaSignature")]
pub struct Signature(pub(crate) libsecp256k1::Signature);

impl Signature {
    /// Serialize the signature into a byte vector
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.serialize().to_vec()
    }

    fn verify(
        &self,
        message: &libsecp256k1::Message,
        public_key: &libsecp256k1::PublicKey,
    ) -> Result<()> {
        // Prevent malleability attacks, low order only. The library only signs in low
        // order, so this was done intentionally.
        if self.0.s.is_high() {
            Err(anyhow!(CryptoMaterialError::CanonicalRepresentationError))
        } else if libsecp256k1::verify(message, &self.0, public_key) {
            Ok(())
        } else {
            Err(anyhow!("Unable to verify signature."))
        }
    }
}

impl Eq for Signature {}

impl PartialEq for Signature {
    fn eq(&self, other: &Signature) -> bool {
        self.to_bytes()[..] == other.to_bytes()[..]
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<Signature, CryptoMaterialError> {
        match libsecp256k1::Signature::parse_standard_slice(bytes) {
            Ok(signature) => Ok(Signature(signature)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "secp256k1_ecdsa::Signature({})", self)
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.to_bytes()[..]))
    }
}

impl std::hash::Hash for Signature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.to_bytes());
    }
}

impl traits::Signature for Signature {
    type SigningKeyMaterial = PrivateKey;
    type VerifyingKeyMaterial = PublicKey;

    fn verify<T: CryptoHash + Serialize>(&self, message: &T, public_key: &PublicKey) -> Result<()> {
        let message = bytes_to_message(&traits::signing_message(message)?)?;
        self.verify(&message, &public_key.0)
    }

    fn verify_arbitrary_msg(&self, message: &[u8], public_key: &PublicKey) -> Result<()> {
        let message = bytes_to_message(message)?;
        self.verify(&message, &public_key.0)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl traits::Length for Signature {
    fn length(&self) -> usize {
        SIGNATURE_LENGTH
    }
}

impl ValidCryptoMaterial for Signature {
    const AIP_80_PREFIX: &'static str = "secp256k1-sig-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

fn bytes_to_message(message: &[u8]) -> Result<libsecp256k1::Message> {
    let message_digest = HashValue::sha3_256_of(message).to_vec();
    libsecp256k1::Message::parse_slice(&message_digest).map_err(|e| anyhow!("{}", e))
}

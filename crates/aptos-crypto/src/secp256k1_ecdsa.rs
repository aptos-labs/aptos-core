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
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

/// Secp256k1 ecdsa public key
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

// floor(n/2) where n is the secp256k1 scalar field order
const SECP256K1_HALF_ORDER_FLOOR: [u32; 8] = [0x681B20A0, 0xDFE92F46, 0x57A4501D, 0x5D576E73, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x7FFFFFFF];

fn as_u32_be(vec: &[u8; 4]) -> u32 {
    println!("vec: {:?}", vec);
    //assert!(vec.len() == 4);
    let i1 = (vec[0] as u32) << 24;
    let i2 = (vec[1] as u32) << 16;
    let i3 = (vec[2] as u32) << 8;
    let i4 = (vec[3] as u32) << 0;
    /*((array[0] as u32) << 24) +
    ((array[1] as u32) << 16) +
    ((array[2] as u32) <<  8) +
    ((array[3] as u32) <<  0)*/
    println!("as_u32_be: {}, {}, {}, {}", i1, i2, i3, i4);
    i1 + i2 + i3 + i4
}


impl Signature {
    /// Serialize the signature into a byte vector
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.serialize().to_vec()
    }

    // Returns true if `s` is equal to floor(n/2), where n is the order of the scalar field of
    // secp256k1
    fn s_equal_half_order_floor(&self) -> bool {
        let s_bytes = self.0.s.b32();
        println!("in s_equal_half_order_floor: {:?}", s_bytes);
        let s_limb_0 = as_u32_be(&s_bytes[0..4].try_into().unwrap());
        println!("did first as_u32_be");
        let s_limb_1 = as_u32_be(&s_bytes[4..8].try_into().unwrap());
        let s_limb_2 = as_u32_be(&s_bytes[8..12].try_into().unwrap());
        let s_limb_3 = as_u32_be(&s_bytes[12..16].try_into().unwrap());
        let s_limb_4 = as_u32_be(&s_bytes[16..20].try_into().unwrap());
        let s_limb_5 = as_u32_be(&s_bytes[20..24].try_into().unwrap());
        let s_limb_6 = as_u32_be(&s_bytes[24..28].try_into().unwrap());
        let s_limb_7 = as_u32_be(&s_bytes[28..32].try_into().unwrap());
        println!("s_limb_0: {}", s_limb_0);
        println!("SECP256K1_HALF_ORDER_FLOOR: {:?}", SECP256K1_HALF_ORDER_FLOOR);
        s_limb_0 == SECP256K1_HALF_ORDER_FLOOR[0]
            && s_limb_1 == SECP256K1_HALF_ORDER_FLOOR[1]
            && s_limb_2 == SECP256K1_HALF_ORDER_FLOOR[2]
            && s_limb_3 == SECP256K1_HALF_ORDER_FLOOR[3]
            && s_limb_4 == SECP256K1_HALF_ORDER_FLOOR[4]
            && s_limb_5 == SECP256K1_HALF_ORDER_FLOOR[5]
            && s_limb_6 == SECP256K1_HALF_ORDER_FLOOR[6]
            && s_limb_7 == SECP256K1_HALF_ORDER_FLOOR[7]
    }

    fn verify(
        &self,
        message: &libsecp256k1::Message,
        public_key: &libsecp256k1::PublicKey,
    ) -> Result<()> {
        println!("s: {:?}", self.0.clone());
        println!("is equal to half order floor: {}", self.s_equal_half_order_floor());
        // Prevent malleability attacks, low order only. The library only signs in low
        // order, so this was done intentionally.
        // The underlying secp256k1 library has a bug - `is_high` should check whether s > n/2.
        // However, it incorrectly returns true when s = floor(n/2), despite the fact that
        // floor(n/2) < n/2. We special case this.
        if self.0.s.is_high() && !self.s_equal_half_order_floor() {
            Err(anyhow!(CryptoMaterialError::CanonicalRepresentationError))
        } else if libsecp256k1::verify(message, &self.0, public_key) {
            Ok(())
        } else {
            /*if self.s_equal_half_order_floor() {
                return Err(anyhow!(CryptoMaterialError::CanonicalRepresentationError));
            }*/
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
        println!("in try from");
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
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

fn bytes_to_message(message: &[u8]) -> Result<libsecp256k1::Message> {
    let message_digest = HashValue::sha3_256_of(message).to_vec();
    libsecp256k1::Message::parse_slice(&message_digest).map_err(|e| anyhow!("{}", e))
}

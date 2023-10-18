// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file implements traits for P256 private keys and public keys.

#[cfg(any(test, feature = "fuzzing"))]
use crate::test_utils::{self, KeyPair};
use crate::{
    hash::CryptoHash,
    p256_ecdsa::{P256Signature, ORDER, P256_PRIVATE_KEY_LENGTH, P256_PUBLIC_KEY_LENGTH},
    traits::*,
};
use aptos_crypto_derive::{DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use core::convert::TryFrom;
use num_bigint::BigUint;
use num_integer::Integer;
use p256::{self, ecdsa::signature::Signer};
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
use serde::Serialize;
use std::fmt;

/// A P256 private key
#[derive(DeserializeKey, SerializeKey, SilentDebug, SilentDisplay)]
pub struct P256PrivateKey(pub(crate) p256::ecdsa::SigningKey);

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(P256PrivateKey: Clone);

#[cfg(any(test, feature = "cloneable-private-keys"))]
impl Clone for P256PrivateKey {
    fn clone(&self) -> Self {
        let serialized: &[u8] = &(self.to_bytes());
        P256PrivateKey::try_from(serialized).unwrap()
    }
}

/// A P256 public key
#[derive(DeserializeKey, Clone, SerializeKey)]
pub struct P256PublicKey(pub(crate) p256::ecdsa::VerifyingKey);

impl P256PrivateKey {
    /// The length of the P256PrivateKey
    pub const LENGTH: usize = P256_PRIVATE_KEY_LENGTH;

    /// Serialize a P256PrivateKey. Uses the SEC1 serialization format.
    pub fn to_bytes(&self) -> [u8; P256_PRIVATE_KEY_LENGTH] {
        self.0.to_bytes().into()
    }

    /// Deserialize an P256PrivateKey without any validation checks apart from expected key size.
    /// Uses the SEC1 serialization format.
    fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<P256PrivateKey, CryptoMaterialError> {
        match p256::ecdsa::SigningKey::from_slice(bytes) {
            Ok(p256_secret_key) => Ok(P256PrivateKey(p256_secret_key)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }

    /// Private function aimed at minimizing code duplication between sign
    /// methods of the SigningKey implementation. This should remain private.
    /// This function uses the `RustCrypto` P256 signing library, which uses,
    /// as of version 0.13.2, SHA2-256 as its hashing algorithm
    fn sign_arbitrary_message(&self, message: &[u8]) -> P256Signature {
        let secret_key = &self.0;
        let sig = P256Signature(secret_key.sign(message.as_ref()));
        P256Signature::make_canonical(&sig)
    }
}

impl P256PublicKey {
    /// Serialize a P256PublicKey. Uses the SEC1 serialization format.
    pub fn to_bytes(&self) -> [u8; P256_PUBLIC_KEY_LENGTH] {
        // The RustCrypto P256 `to_sec1_bytes` call here should never return an array of the wrong length and cause a panic
        (*self.0.to_sec1_bytes()).try_into().unwrap()
    }

    /// Deserialize a P256PublicKey, checking expected key size
    /// and that it is a valid curve point.
    /// Uses the SEC1 serialization format.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<P256PublicKey, CryptoMaterialError> {
        match p256::ecdsa::VerifyingKey::from_sec1_bytes(bytes) {
            Ok(p256_public_key) => Ok(P256PublicKey(p256_public_key)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }
}

///////////////////////
// PrivateKey Traits //
///////////////////////

impl PrivateKey for P256PrivateKey {
    type PublicKeyMaterial = P256PublicKey;
}

impl SigningKey for P256PrivateKey {
    type SignatureMaterial = P256Signature;
    type VerifyingKeyMaterial = P256PublicKey;

    fn sign<T: CryptoHash + Serialize>(
        &self,
        message: &T,
    ) -> Result<P256Signature, CryptoMaterialError> {
        Ok(P256PrivateKey::sign_arbitrary_message(
            self,
            signing_message(message)?.as_ref(),
        ))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn sign_arbitrary_message(&self, message: &[u8]) -> P256Signature {
        P256PrivateKey::sign_arbitrary_message(self, message)
    }
}

impl Uniform for P256PrivateKey {
    // Returns a random field element as a private key indistinguishable from uniformly random.
    // Uses a hack to get around the incompatability of the `aptos-crypto` RngCore trait and the
    // `RustCrypto` RngCore trait
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore,
    {
        let mut bytes = [0u8; P256_PRIVATE_KEY_LENGTH * 2];
        rng.fill_bytes(&mut bytes);
        let bignum = BigUint::from_bytes_be(&bytes[..]);
        let order = BigUint::from_bytes_be(&ORDER);
        let remainder = bignum.mod_floor(&order);
        P256PrivateKey::from_bytes_unchecked(&remainder.to_bytes_be()).unwrap()
    }
}

impl PartialEq<Self> for P256PrivateKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for P256PrivateKey {}

impl TryFrom<&[u8]> for P256PrivateKey {
    type Error = CryptoMaterialError;

    /// Deserialize a P256PrivateKey. This method will check for private key validity: i.e.,
    /// correct key length.
    fn try_from(bytes: &[u8]) -> std::result::Result<P256PrivateKey, CryptoMaterialError> {
        // Note that the only requirement is that the size of the key is 32 bytes, something that
        // is already checked during deserialization of p256::ecdsa::SigningKey
        P256PrivateKey::from_bytes_unchecked(bytes)
    }
}

impl Length for P256PrivateKey {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl ValidCryptoMaterial for P256PrivateKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Genesis for P256PrivateKey {
    fn genesis() -> Self {
        let mut buf = [0u8; P256_PRIVATE_KEY_LENGTH];
        buf[P256_PRIVATE_KEY_LENGTH - 1] = 1;
        Self::try_from(buf.as_ref()).unwrap()
    }
}

//////////////////////
// PublicKey Traits //
//////////////////////

// Implementing From<&PrivateKey<...>> allows to derive a public key in a more elegant fashion
impl From<&P256PrivateKey> for P256PublicKey {
    fn from(private_key: &P256PrivateKey) -> Self {
        let secret = &private_key.0;
        let public: p256::ecdsa::VerifyingKey = secret.into();
        P256PublicKey(public)
    }
}

// We deduce PublicKey from this
impl PublicKey for P256PublicKey {
    type PrivateKeyMaterial = P256PrivateKey;
}

impl std::hash::Hash for P256PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_pubkey = self.to_bytes();
        state.write(&encoded_pubkey);
    }
}

// Those are required by the implementation of hash above
impl PartialEq for P256PublicKey {
    fn eq(&self, other: &P256PublicKey) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for P256PublicKey {}

// We deduce VerifyingKey from pointing to the signature material
// we get the ability to do `pubkey.validate(msg, signature)`
impl VerifyingKey for P256PublicKey {
    type SignatureMaterial = P256Signature;
    type SigningKeyMaterial = P256PrivateKey;
}

impl fmt::Display for P256PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.to_sec1_bytes()))
    }
}

impl fmt::Debug for P256PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P256PublicKey({})", self)
    }
}

impl TryFrom<&[u8]> for P256PublicKey {
    type Error = CryptoMaterialError;

    /// Deserialize a P256PublicKey.
    fn try_from(bytes: &[u8]) -> std::result::Result<P256PublicKey, CryptoMaterialError> {
        P256PublicKey::from_bytes_unchecked(bytes)
    }
}

impl Length for P256PublicKey {
    fn length(&self) -> usize {
        P256_PUBLIC_KEY_LENGTH
    }
}

impl ValidCryptoMaterial for P256PublicKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

/////////////
// Fuzzing //
/////////////

/// Produces a uniformly random P256 keypair from a seed
#[cfg(any(test, feature = "fuzzing"))]
pub fn keypair_strategy() -> impl Strategy<Value = KeyPair<P256PrivateKey, P256PublicKey>> {
    test_utils::uniform_keypair_strategy::<P256PrivateKey, P256PublicKey>()
}

/// Produces a uniformly random P256 public key
#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for P256PublicKey {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        crate::test_utils::uniform_keypair_strategy::<P256PrivateKey, P256PublicKey>()
            .prop_map(|v| v.public_key)
            .boxed()
    }
}

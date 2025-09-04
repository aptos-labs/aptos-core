// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file implements traits for Secp256r1 ECDSA private keys and public keys.

#[cfg(any(test, feature = "fuzzing"))]
use crate::test_utils::{self, KeyPair};
use crate::{
    hash::CryptoHash,
    secp256r1_ecdsa::{Signature, ORDER, PRIVATE_KEY_LENGTH, PUBLIC_KEY_LENGTH},
    traits::{PrivateKey as PrivateKeyTrait, PublicKey as PublicKeyTrait, *},
};
use velor_crypto_derive::{key_name, DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use core::convert::TryFrom;
use num_bigint::BigUint;
use num_integer::Integer;
use p256::{self, ecdsa::signature::Signer};
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
use serde::Serialize;
use std::fmt;

/// A secp256r1_ecdsa private key
#[derive(DeserializeKey, SerializeKey, SilentDebug, SilentDisplay)]
#[key_name("Secp256r1EcdsaPrivateKey")]
pub struct PrivateKey(pub(crate) p256::ecdsa::SigningKey);

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(PrivateKey: Clone);

#[cfg(any(test, feature = "cloneable-private-keys"))]
impl Clone for PrivateKey {
    fn clone(&self) -> Self {
        let serialized: &[u8] = &(self.to_bytes());
        PrivateKey::try_from(serialized).unwrap()
    }
}

/// A secp256r1_ecdsa public key
#[derive(DeserializeKey, Clone, SerializeKey)]
#[key_name("Secp256r1EcdsaPublicKey")]
pub struct PublicKey(pub(crate) p256::ecdsa::VerifyingKey);

#[cfg(any(test, feature = "fuzzing"))]
impl<'a> arbitrary::Arbitrary<'a> for PublicKey {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let bytes: [u8; PUBLIC_KEY_LENGTH] = u.arbitrary()?;
        PublicKey::from_bytes_unchecked(&bytes).map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}

impl PrivateKey {
    /// The length of the PrivateKey
    pub const LENGTH: usize = PRIVATE_KEY_LENGTH;

    /// Serialize a PrivateKey. Uses the SEC1 serialization format.
    pub fn to_bytes(&self) -> [u8; PRIVATE_KEY_LENGTH] {
        self.0.to_bytes().into()
    }

    /// Deserialize a PrivateKey without any validation checks apart from expected key size.
    /// Uses the SEC1 serialization format. Bytes are expected to be in big-endian form.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<PrivateKey, CryptoMaterialError> {
        match p256::ecdsa::SigningKey::from_slice(bytes) {
            Ok(p256_secret_key) => Ok(PrivateKey(p256_secret_key)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }

    /// Private function aimed at minimizing code duplication between sign
    /// methods of the SigningKey implementation. This should remain private.
    /// This function uses the `RustCrypto` secp256r1_ecdsa signing library, which uses,
    /// as of version 0.13.2, SHA2-256 as its hashing algorithm
    fn sign_arbitrary_message(&self, message: &[u8]) -> Signature {
        let secret_key = &self.0;
        let sig = Signature(secret_key.sign(message.as_ref()));
        Signature::make_canonical(&sig)
    }
}

impl PublicKey {
    /// Serialize a PublicKey. Uses the SEC1 serialization format.
    pub fn to_bytes(&self) -> [u8; PUBLIC_KEY_LENGTH] {
        // The RustCrypto P256 `to_sec1_bytes` call here should never return an array of the wrong length and cause a panic
        (*self.0.to_sec1_bytes()).try_into().unwrap()
    }

    /// Deserialize a P256PublicKey, checking expected key size
    /// and that it is a valid curve point.
    /// Uses the SEC1 serialization format.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<PublicKey, CryptoMaterialError> {
        match p256::ecdsa::VerifyingKey::from_sec1_bytes(bytes) {
            Ok(p256_public_key) => Ok(PublicKey(p256_public_key)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }
}

///////////////////////
// PrivateKey Traits //
///////////////////////

impl PrivateKeyTrait for PrivateKey {
    type PublicKeyMaterial = PublicKey;
}

impl SigningKey for PrivateKey {
    type SignatureMaterial = Signature;
    type VerifyingKeyMaterial = PublicKey;

    fn sign<T: CryptoHash + Serialize>(
        &self,
        message: &T,
    ) -> Result<Signature, CryptoMaterialError> {
        Ok(PrivateKey::sign_arbitrary_message(
            self,
            signing_message(message)?.as_ref(),
        ))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn sign_arbitrary_message(&self, message: &[u8]) -> Signature {
        PrivateKey::sign_arbitrary_message(self, message)
    }
}

impl Uniform for PrivateKey {
    // Returns a random field element as a private key indistinguishable from uniformly random.
    // Uses a hack to get around the incompatability of the `velor-crypto` RngCore trait and the
    // `RustCrypto` RngCore trait
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore,
    {
        let mut bytes = [0u8; PRIVATE_KEY_LENGTH * 2];
        rng.fill_bytes(&mut bytes);
        let bignum = BigUint::from_bytes_be(&bytes[..]);
        let order = BigUint::from_bytes_be(&ORDER);
        let remainder = bignum.mod_floor(&order);
        PrivateKey::from_bytes_unchecked(&remainder.to_bytes_be()).unwrap()
    }
}

impl PartialEq<Self> for PrivateKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for PrivateKey {}

impl TryFrom<&[u8]> for PrivateKey {
    type Error = CryptoMaterialError;

    /// Deserialize a PrivateKey. This method will check for private key validity: i.e.,
    /// correct key length.
    fn try_from(bytes: &[u8]) -> std::result::Result<PrivateKey, CryptoMaterialError> {
        // Note that the only requirement is that the size of the key is 32 bytes, something that
        // is already checked during deserialization of p256::ecdsa::SigningKey
        PrivateKey::from_bytes_unchecked(bytes)
    }
}

impl Length for PrivateKey {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl ValidCryptoMaterial for PrivateKey {
    const AIP_80_PREFIX: &'static str = "secp256r1-priv-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Genesis for PrivateKey {
    fn genesis() -> Self {
        let mut buf = [0u8; PRIVATE_KEY_LENGTH];
        buf[PRIVATE_KEY_LENGTH - 1] = 1;
        Self::try_from(buf.as_ref()).unwrap()
    }
}

//////////////////////
// PublicKey Traits //
//////////////////////

// Implementing From<&PrivateKey<...>> allows to derive a public key in a more elegant fashion
impl From<&PrivateKey> for PublicKey {
    fn from(private_key: &PrivateKey) -> Self {
        let secret = &private_key.0;
        let public: p256::ecdsa::VerifyingKey = secret.into();
        PublicKey(public)
    }
}

// We deduce PublicKey from this
impl PublicKeyTrait for PublicKey {
    type PrivateKeyMaterial = PrivateKey;
}

impl std::hash::Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_pubkey = self.to_bytes();
        state.write(&encoded_pubkey);
    }
}

// Those are required by the implementation of hash above
impl PartialEq for PublicKey {
    fn eq(&self, other: &PublicKey) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for PublicKey {}

// We deduce VerifyingKey from pointing to the signature material
// we get the ability to do `pubkey.validate(msg, signature)`
impl VerifyingKey for PublicKey {
    type SignatureMaterial = Signature;
    type SigningKeyMaterial = PrivateKey;
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.to_sec1_bytes()))
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "secp256r1_ecdsa::PublicKey({})", self)
    }
}

impl TryFrom<&[u8]> for PublicKey {
    type Error = CryptoMaterialError;

    /// Deserialize a PublicKey.
    fn try_from(bytes: &[u8]) -> std::result::Result<PublicKey, CryptoMaterialError> {
        PublicKey::from_bytes_unchecked(bytes)
    }
}

impl Length for PublicKey {
    fn length(&self) -> usize {
        PUBLIC_KEY_LENGTH
    }
}

impl ValidCryptoMaterial for PublicKey {
    const AIP_80_PREFIX: &'static str = "secp256r1-pub-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

/////////////
// Fuzzing //
/////////////

/// Produces a uniformly random secp256r1_ecdsa keypair from a seed
#[cfg(any(test, feature = "fuzzing"))]
pub fn keypair_strategy() -> impl Strategy<Value = KeyPair<PrivateKey, PublicKey>> {
    test_utils::uniform_keypair_strategy::<PrivateKey, PublicKey>()
}

/// Produces a uniformly random secp256r1_ecdsa public key
#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for PublicKey {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        crate::test_utils::uniform_keypair_strategy::<PrivateKey, PublicKey>()
            .prop_map(|v| v.public_key)
            .boxed()
    }
}

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file implements traits for Ed25519 private keys and public keys.

#[cfg(any(test, feature = "fuzzing"))]
use crate::test_utils::{self, KeyPair};
use crate::{
    ed25519::{Ed25519Signature, ED25519_PRIVATE_KEY_LENGTH, ED25519_PUBLIC_KEY_LENGTH},
    hash::CryptoHash,
    traits::*,
};
use velor_crypto_derive::{DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use core::convert::TryFrom;
use curve25519_dalek::{edwards::CompressedEdwardsY, scalar::Scalar};
use ed25519_dalek::ExpandedSecretKey;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
use serde::Serialize;
use std::fmt;

/// An Ed25519 private key
#[derive(DeserializeKey, SerializeKey, SilentDebug, SilentDisplay)]
pub struct Ed25519PrivateKey(pub(crate) ed25519_dalek::SecretKey);

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(Ed25519PrivateKey: Clone);

#[cfg(any(test, feature = "cloneable-private-keys"))]
impl Clone for Ed25519PrivateKey {
    fn clone(&self) -> Self {
        let serialized: &[u8] = &(self.to_bytes());
        Ed25519PrivateKey::try_from(serialized).unwrap()
    }
}

/// An Ed25519 public key
#[derive(DeserializeKey, Clone, SerializeKey)]
pub struct Ed25519PublicKey(pub(crate) ed25519_dalek::PublicKey);

#[cfg(any(test, feature = "fuzzing"))]
impl<'a> arbitrary::Arbitrary<'a> for Ed25519PublicKey {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let bytes: [u8; ED25519_PUBLIC_KEY_LENGTH] = u.arbitrary()?;
        Ed25519PublicKey::from_bytes_unchecked(&bytes)
            .map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}

impl Ed25519PrivateKey {
    /// The length of the Ed25519PrivateKey
    pub const LENGTH: usize = ED25519_PRIVATE_KEY_LENGTH;

    /// Serialize an Ed25519PrivateKey.
    pub fn to_bytes(&self) -> [u8; ED25519_PRIVATE_KEY_LENGTH] {
        self.0.to_bytes()
    }

    /// Deserialize an Ed25519PrivateKey without any validation checks apart from expected key size.
    fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<Ed25519PrivateKey, CryptoMaterialError> {
        match ed25519_dalek::SecretKey::from_bytes(bytes) {
            Ok(dalek_secret_key) => Ok(Ed25519PrivateKey(dalek_secret_key)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }

    /// Private function aimed at minimizing code duplication between sign
    /// methods of the SigningKey implementation. This should remain private.
    fn sign_arbitrary_message(&self, message: &[u8]) -> Ed25519Signature {
        let secret_key: &ed25519_dalek::SecretKey = &self.0;
        let public_key: Ed25519PublicKey = self.into();
        let expanded_secret_key: ed25519_dalek::ExpandedSecretKey =
            ed25519_dalek::ExpandedSecretKey::from(secret_key);
        let sig = expanded_secret_key.sign(message.as_ref(), &public_key.0);
        Ed25519Signature(sig)
    }

    /// Derive the actual scalar represented by the secret key.
    /// TODO: We are temporarily breaking the abstraction here and exposing the SK scalar. In the future, we should add traits for encryption inside velor-crypto so that we can both sign and decrypt with an Ed25519PrivateKey.
    pub fn derive_scalar(&self) -> Scalar {
        let expanded_bytes = ExpandedSecretKey::from(&self.0).to_bytes();
        let bits = expanded_bytes[..32]
            .try_into()
            .expect("converting [u8; 64] to [u8; 32] should work");
        Scalar::from_bits(bits).reduce()
    }
}

impl Ed25519PublicKey {
    /// The maximum size in bytes.
    pub const LENGTH: usize = ED25519_PUBLIC_KEY_LENGTH;

    /// Serialize an Ed25519PublicKey.
    pub fn to_bytes(&self) -> [u8; ED25519_PUBLIC_KEY_LENGTH] {
        self.0.to_bytes()
    }

    /// Deserialize an Ed25519PublicKey without any validation checks apart from expected key size
    /// and valid curve point, although not necessarily in the prime-order subgroup.
    ///
    /// This function does NOT check the public key for membership in a small subgroup.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<Ed25519PublicKey, CryptoMaterialError> {
        match ed25519_dalek::PublicKey::from_bytes(bytes) {
            Ok(dalek_public_key) => Ok(Ed25519PublicKey(dalek_public_key)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }

    /// Deserialize an Ed25519PublicKey from its representation as an x25519
    /// public key, along with an indication of sign. This is meant to
    /// compensate for the poor key storage capabilities of key management
    /// solutions, and NOT to promote double usage of keys under several
    /// schemes, which would lead to BAD vulnerabilities.
    ///
    /// This function does NOT check if the public key lies in a small subgroup.
    ///
    /// Arguments:
    /// - `x25519_bytes`: bit representation of a public key in clamped
    ///            Montgomery form, a.k.a. the x25519 public key format.
    /// - `negative`: whether to interpret the given point as a negative point,
    ///               as the Montgomery form erases the sign byte. By XEdDSA
    ///               convention, if you expect to ever convert this back to an
    ///               x25519 public key, you should pass `false` for this
    ///               argument.
    #[cfg(test)]
    pub(crate) fn from_x25519_public_bytes(
        x25519_bytes: &[u8],
        negative: bool,
    ) -> Result<Self, CryptoMaterialError> {
        if x25519_bytes.len() != 32 {
            return Err(CryptoMaterialError::DeserializationError);
        }
        let key_bits = {
            let mut bits = [0u8; 32];
            bits.copy_from_slice(x25519_bytes);
            bits
        };
        let mtg_point = curve25519_dalek::montgomery::MontgomeryPoint(key_bits);
        let sign = u8::from(negative);
        let ed_point = mtg_point
            .to_edwards(sign)
            .ok_or(CryptoMaterialError::DeserializationError)?;
        Ed25519PublicKey::try_from(&ed_point.compress().as_bytes()[..])
    }

    /// Derive the actual curve point represented by the public key.
    pub fn to_compressed_edwards_y(&self) -> CompressedEdwardsY {
        let bytes = self.to_bytes();
        CompressedEdwardsY::from_slice(bytes.as_slice())
    }
}

///////////////////////
// PrivateKey Traits //
///////////////////////

impl PrivateKey for Ed25519PrivateKey {
    type PublicKeyMaterial = Ed25519PublicKey;
}

impl SigningKey for Ed25519PrivateKey {
    type SignatureMaterial = Ed25519Signature;
    type VerifyingKeyMaterial = Ed25519PublicKey;

    fn sign<T: CryptoHash + Serialize>(
        &self,
        message: &T,
    ) -> Result<Ed25519Signature, CryptoMaterialError> {
        Ok(Ed25519PrivateKey::sign_arbitrary_message(
            self,
            signing_message(message)?.as_ref(),
        ))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn sign_arbitrary_message(&self, message: &[u8]) -> Ed25519Signature {
        Ed25519PrivateKey::sign_arbitrary_message(self, message)
    }
}

impl Uniform for Ed25519PrivateKey {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore,
    {
        Ed25519PrivateKey(ed25519_dalek::SecretKey::generate(rng))
    }
}

impl PartialEq<Self> for Ed25519PrivateKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for Ed25519PrivateKey {}

// We could have a distinct kind of validation for the PrivateKey: e.g., checking the derived
// PublicKey is valid?
impl TryFrom<&[u8]> for Ed25519PrivateKey {
    type Error = CryptoMaterialError;

    /// Deserialize an Ed25519PrivateKey. This method will check for private key validity: i.e.,
    /// correct key length.
    fn try_from(bytes: &[u8]) -> std::result::Result<Ed25519PrivateKey, CryptoMaterialError> {
        // Note that the only requirement is that the size of the key is 32 bytes, something that
        // is already checked during deserialization of ed25519_dalek::SecretKey
        //
        // Also, the underlying ed25519_dalek implementation ensures that the derived public key
        // is safe and it will not lie in a small-order group, thus no extra check for PublicKey
        // validation is required.
        Ed25519PrivateKey::from_bytes_unchecked(bytes)
    }
}

impl Length for Ed25519PrivateKey {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl ValidCryptoMaterial for Ed25519PrivateKey {
    const AIP_80_PREFIX: &'static str = "ed25519-priv-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Genesis for Ed25519PrivateKey {
    fn genesis() -> Self {
        let mut buf = [0u8; ED25519_PRIVATE_KEY_LENGTH];
        buf[ED25519_PRIVATE_KEY_LENGTH - 1] = 1;
        Self::try_from(buf.as_ref()).unwrap()
    }
}

//////////////////////
// PublicKey Traits //
//////////////////////

// Implementing From<&PrivateKey<...>> allows to derive a public key in a more elegant fashion
impl From<&Ed25519PrivateKey> for Ed25519PublicKey {
    fn from(private_key: &Ed25519PrivateKey) -> Self {
        let secret: &ed25519_dalek::SecretKey = &private_key.0;
        let public: ed25519_dalek::PublicKey = secret.into();
        Ed25519PublicKey(public)
    }
}

// We deduce PublicKey from this
impl PublicKey for Ed25519PublicKey {
    type PrivateKeyMaterial = Ed25519PrivateKey;
}

impl std::hash::Hash for Ed25519PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_pubkey = self.to_bytes();
        state.write(&encoded_pubkey);
    }
}

// Those are required by the implementation of hash above
impl PartialEq for Ed25519PublicKey {
    fn eq(&self, other: &Ed25519PublicKey) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for Ed25519PublicKey {}

// We deduce VerifyingKey from pointing to the signature material
// we get the ability to do `pubkey.validate(msg, signature)`
impl VerifyingKey for Ed25519PublicKey {
    type SignatureMaterial = Ed25519Signature;
    type SigningKeyMaterial = Ed25519PrivateKey;
}

impl fmt::Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.as_bytes()))
    }
}

impl fmt::Debug for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519PublicKey({})", self)
    }
}

impl TryFrom<&[u8]> for Ed25519PublicKey {
    type Error = CryptoMaterialError;

    /// Deserialize an Ed25519PublicKey. This method will NOT check for key validity, which means
    /// the returned public key could be in a small subgroup. Nonetheless, our signature
    /// verification implicitly checks if the public key lies in a small subgroup, so canonical
    /// uses of this library will not be susceptible to small subgroup attacks.
    fn try_from(bytes: &[u8]) -> std::result::Result<Ed25519PublicKey, CryptoMaterialError> {
        Ed25519PublicKey::from_bytes_unchecked(bytes)
    }
}

impl Length for Ed25519PublicKey {
    fn length(&self) -> usize {
        ED25519_PUBLIC_KEY_LENGTH
    }
}

impl ValidCryptoMaterial for Ed25519PublicKey {
    const AIP_80_PREFIX: &'static str = "ed25519-pub-";

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }
}

/////////////
// Fuzzing //
/////////////

/// Produces a uniformly random Ed25519 keypair from a seed
#[cfg(any(test, feature = "fuzzing"))]
pub fn keypair_strategy() -> impl Strategy<Value = KeyPair<Ed25519PrivateKey, Ed25519PublicKey>> {
    test_utils::uniform_keypair_strategy::<Ed25519PrivateKey, Ed25519PublicKey>()
}

/// Produces a uniformly random Ed25519 public key
#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for Ed25519PublicKey {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        crate::test_utils::uniform_keypair_strategy::<Ed25519PrivateKey, Ed25519PublicKey>()
            .prop_map(|v| v.public_key)
            .boxed()
    }
}

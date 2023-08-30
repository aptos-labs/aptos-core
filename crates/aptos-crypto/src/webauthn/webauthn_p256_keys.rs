// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file implements traits for WebAuthn P256 private keys and public keys.

#[cfg(any(test, feature = "fuzzing"))]
use crate::test_utils::{self, KeyPair};
use crate::{
    hash::CryptoHash,
    p256_ecdsa::{P256PrivateKey, P256PublicKey, P256_PUBLIC_KEY_LENGTH},
    traits::*,
    webauthn::webauthn_p256_sigs::WebAuthnP256Signature,
};
use aptos_crypto_derive::{DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use core::convert::TryFrom;
use p256::{self, ecdsa};
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
use serde::Serialize;
use std::fmt;


/// A WebAuthn P256 private key
#[derive(DeserializeKey, SerializeKey, SilentDebug, SilentDisplay)]
pub struct WebAuthnP256PrivateKey(pub(crate) P256PrivateKey);

impl private::Sealed for WebAuthnP256PrivateKey {}

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(WebAuthnP256PrivateKey: Clone);

#[cfg(any(test, feature = "cloneable-private-keys"))]
impl Clone for WebAuthnP256PrivateKey {
    fn clone(&self) -> Self {
        let serialized: &[u8] = &(self.to_bytes());
        WebAuthnP256PrivateKey::try_from(serialized).unwrap()
    }
}

/// A WebAuthn P256 public key
#[derive(DeserializeKey, Clone, SerializeKey)]
pub struct WebAuthnP256PublicKey(pub(crate) P256PublicKey);

impl private::Sealed for WebAuthnP256PublicKey {}

impl WebAuthnP256PrivateKey {
    /// The length of the WebAuthnP256PrivateKey
    pub const LENGTH: usize = P256PrivateKey::LENGTH;

    /// Serialize a WebAuthnP256PrivateKey.
    pub fn to_bytes(&self) -> [u8; P256PrivateKey::LENGTH] {
        self.0.to_bytes()
    }

    /// Deserialize a WebAuthnP256PrivateKey without any validation checks apart from expected key size.
    fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<WebAuthnP256PrivateKey, CryptoMaterialError> {
        match p256::ecdsa::SigningKey::from_slice(bytes) {
            Ok(p256_secret_key) => Ok(WebAuthnP256PrivateKey(P256PrivateKey(p256_secret_key))),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }

    /// This may be problematic
    ///
    /// Trait method is required but does NOT work well for WebAuthn signatures
    /// because auth_data and client_data_json are not known. Normally, this method is expected to return
    /// a `WebAuthnP256Signature` which is supposed to include both of those fields.
    ///
    /// Additionally, a `P256Signature` cannot be returned as this results in a type mismatch
    /// error resolving `<P256Signature as Signature>::SigningKeyMaterial == WebAuthnP256PrivateKey`
    ///
    /// Note also that these raw, fixed byte P256 signatures are different from the ASN.1 DER encoded
    /// signatures used in the WebAuthn specification.
    ///
    /// WebAuthn private keys are never exposed to the user, so in practice
    /// this would never happen. It is just meant to be a dummy signer function
    ///
    /// More info: [WebAuthn §7.2](https://www.w3.org/TR/webauthn-3/#sctn-verifying-assertion)
    ///
    /// Private function aimed at minimizing code duplication between sign
    /// methods of the SigningKey implementation. This should remain private.
    fn sign_arbitrary_message(&self, _message: &[u8]) -> WebAuthnP256Signature {
        WebAuthnP256Signature(vec![])
    }
}

impl WebAuthnP256PublicKey {
    /// Serialize a WebAuthnP256PublicKey.
    pub fn to_bytes(&self) -> [u8; P256_PUBLIC_KEY_LENGTH] {
        self.0.to_bytes()
    }

    /// Deserialize a WebAuthnP256PublicKey, checking expected key size
    /// and that it is a valid curve point.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> Result<WebAuthnP256PublicKey, CryptoMaterialError> {
        match ecdsa::VerifyingKey::from_sec1_bytes(bytes) {
            Ok(p256_public_key) => Ok(WebAuthnP256PublicKey(P256PublicKey(p256_public_key))),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }
}

///////////////////////
// PrivateKey Traits //
///////////////////////

impl PrivateKey for WebAuthnP256PrivateKey {
    type PublicKeyMaterial = WebAuthnP256PublicKey;
}

impl SigningKey for WebAuthnP256PrivateKey {
    type VerifyingKeyMaterial = WebAuthnP256PublicKey;
    type SignatureMaterial = WebAuthnP256Signature;

    fn sign<T: CryptoHash + Serialize>(
        &self,
        message: &T,
    ) -> Result<WebAuthnP256Signature, CryptoMaterialError> {
        Ok(WebAuthnP256PrivateKey::sign_arbitrary_message(
            self,
            signing_message(message)?.as_ref(),
        ))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn sign_arbitrary_message(&self, message: &[u8]) -> WebAuthnP256Signature {
        WebAuthnP256PrivateKey::sign_arbitrary_message(self, message)
    }
}

impl Uniform for WebAuthnP256PrivateKey {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore,
    {
        let mut bytes: [u8; P256PrivateKey::LENGTH] = Default::default();
        rng.fill_bytes(&mut bytes);
        WebAuthnP256PrivateKey(P256PrivateKey(
            p256::ecdsa::SigningKey::from_slice(&bytes[..]).unwrap(),
        ))
    }
}

impl PartialEq<Self> for WebAuthnP256PrivateKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for WebAuthnP256PrivateKey {}

// We could have a distinct kind of validation for the PrivateKey: e.g., checking the derived
// PublicKey is valid?
impl TryFrom<&[u8]> for WebAuthnP256PrivateKey {
    type Error = CryptoMaterialError;

    /// Deserialize a WebAuthnP256PrivateKey. This method will check for private key validity: i.e.,
    /// correct key length.
    fn try_from(bytes: &[u8]) -> std::result::Result<WebAuthnP256PrivateKey, CryptoMaterialError> {
        // Note that the only requirement is that the size of the key is 32 bytes, something that
        // is already checked during deserialization of p256::ecdsa::SigningKey
        WebAuthnP256PrivateKey::from_bytes_unchecked(bytes)
    }
}

impl Length for WebAuthnP256PrivateKey {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl ValidCryptoMaterial for WebAuthnP256PrivateKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Genesis for WebAuthnP256PrivateKey {
    fn genesis() -> Self {
        let mut buf = [0u8; P256PrivateKey::LENGTH];
        buf[P256PrivateKey::LENGTH - 1] = 1;
        Self::try_from(buf.as_ref()).unwrap()
    }
}

//////////////////////
// PublicKey Traits //
//////////////////////

// Implementing From<&PrivateKey<...>> allows to derive a public key in a more elegant fashion
impl From<&WebAuthnP256PrivateKey> for WebAuthnP256PublicKey {
    fn from(private_key: &WebAuthnP256PrivateKey) -> Self {
        let secret = &private_key.0 .0;
        let public: p256::ecdsa::VerifyingKey = secret.into();
        WebAuthnP256PublicKey(P256PublicKey(public))
    }
}

// We deduce PublicKey from this
impl PublicKey for WebAuthnP256PublicKey {
    type PrivateKeyMaterial = WebAuthnP256PrivateKey;
}

impl std::hash::Hash for WebAuthnP256PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_pubkey = self.to_bytes();
        state.write(&encoded_pubkey);
    }
}

// Those are required by the implementation of hash above
impl PartialEq for WebAuthnP256PublicKey {
    fn eq(&self, other: &WebAuthnP256PublicKey) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl Eq for WebAuthnP256PublicKey {}

// We deduce VerifyingKey from pointing to the signature material
// we get the ability to do `pubkey.validate(msg, signature)`
impl VerifyingKey for WebAuthnP256PublicKey {
    type SignatureMaterial = WebAuthnP256Signature;
    type SigningKeyMaterial = WebAuthnP256PrivateKey;
}

impl fmt::Display for WebAuthnP256PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0 .0.to_sec1_bytes()))
    }
}

impl fmt::Debug for WebAuthnP256PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WebAuthnP256PublicKey({})", self)
    }
}

impl TryFrom<&[u8]> for WebAuthnP256PublicKey {
    type Error = CryptoMaterialError;

    /// Deserialize a WebAuthnP256PublicKey.
    fn try_from(bytes: &[u8]) -> std::result::Result<WebAuthnP256PublicKey, CryptoMaterialError> {
        WebAuthnP256PublicKey::from_bytes_unchecked(bytes)
    }
}

impl Length for WebAuthnP256PublicKey {
    fn length(&self) -> usize {
        P256_PUBLIC_KEY_LENGTH
    }
}

impl ValidCryptoMaterial for WebAuthnP256PublicKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.0 .0.to_sec1_bytes().to_vec()
    }
}

/////////////
// Fuzzing //
/////////////

/// Produces a uniformly random P256 keypair from a seed
#[cfg(any(test, feature = "fuzzing"))]
pub fn keypair_strategy(
) -> impl Strategy<Value = KeyPair<WebAuthnP256PrivateKey, WebAuthnP256PublicKey>> {
    test_utils::uniform_keypair_strategy::<WebAuthnP256PrivateKey, WebAuthnP256PublicKey>()
}

/// Produces a uniformly random P256 public key
#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for WebAuthnP256PublicKey {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        crate::test_utils::uniform_keypair_strategy::<WebAuthnP256PrivateKey, WebAuthnP256PublicKey>()
            .prop_map(|v| v.public_key)
            .boxed()
    }
}

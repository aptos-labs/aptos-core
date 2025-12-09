// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This file implements traits for SLH-DSA SHA2-128s private keys and public keys.

#[cfg(any(test, feature = "fuzzing"))]
use crate::test_utils::{self, KeyPair};
use crate::{
    hash::CryptoHash,
    slh_dsa_sha2_128s::{Signature, PRIVATE_KEY_LENGTH, PUBLIC_KEY_LENGTH},
    traits::{PrivateKey as PrivateKeyTrait, PublicKey as PublicKeyTrait, *},
};
use aptos_crypto_derive::{key_name, DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use core::convert::TryFrom;
use serde::Serialize;
use slh_dsa::{Sha2_128s, SigningKey as SlhDsaSigningKey, VerifyingKey as SlhDsaVerifyingKey};
use std::fmt;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;

/// A SLH-DSA SHA2-128s private key (signing key)
#[derive(DeserializeKey, SerializeKey, SilentDebug, SilentDisplay)]
#[key_name("SlhDsaSha2_128sPrivateKey")]
pub struct PrivateKey(pub(crate) SlhDsaSigningKey<Sha2_128s>);

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(PrivateKey: Clone);

#[cfg(any(test, feature = "cloneable-private-keys"))]
impl Clone for PrivateKey {
    fn clone(&self) -> Self {
        let serialized: &[u8] = &(self.to_bytes());
        PrivateKey::try_from(serialized).unwrap()
    }
}

/// A SLH-DSA SHA2-128s public key (verifying key)
#[derive(DeserializeKey, Clone, SerializeKey)]
#[key_name("SlhDsaSha2_128sPublicKey")]
pub struct PublicKey(pub(crate) SlhDsaVerifyingKey<Sha2_128s>);

#[cfg(any(test, feature = "fuzzing"))]
impl<'a> arbitrary::Arbitrary<'a> for PublicKey {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let bytes: Vec<u8> = u.arbitrary()?;
        if bytes.len() == PUBLIC_KEY_LENGTH {
            PublicKey::from_bytes_unchecked(&bytes).map_err(|_| arbitrary::Error::IncorrectFormat)
        } else {
            Err(arbitrary::Error::IncorrectFormat)
        }
    }
}

impl PrivateKey {
    /// The length of the PrivateKey
    pub const LENGTH: usize = PRIVATE_KEY_LENGTH;

    /// Serialize a PrivateKey
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }

    /// Deserialize a PrivateKey without any validation checks apart from expected key size.
    /// For SLH-DSA, we use slh_keygen_internal with the seed as sk_seed, sk_prf, and pk_seed.
    /// Since we only have a 32-byte seed, we use it for all three parameters.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<PrivateKey, CryptoMaterialError> {
        if bytes.len() != PRIVATE_KEY_LENGTH {
            return Err(CryptoMaterialError::WrongLengthError);
        }
        // SLH-DSA private key generation requires sk_seed, sk_prf, and pk_seed
        // For simplicity, we use the same seed for all three (this is a common pattern)
        let signing_key = SlhDsaSigningKey::<Sha2_128s>::slh_keygen_internal(bytes, bytes, bytes);
        Ok(PrivateKey(signing_key))
    }

    /// Private function aimed at minimizing code duplication between sign
    /// methods of the SigningKey implementation. This should remain private.
    fn sign_arbitrary_message(&self, message: &[u8]) -> Signature {
        use slh_dsa::signature::Signer;
        let signature = Signer::<slh_dsa::Signature<Sha2_128s>>::sign(&self.0, message);
        Signature(signature)
    }
}

impl PublicKey {
    /// Serialize a PublicKey
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }

    /// Deserialize a PublicKey, checking expected key size
    /// and that it is a valid public key.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<PublicKey, CryptoMaterialError> {
        if bytes.len() != PUBLIC_KEY_LENGTH {
            return Err(CryptoMaterialError::WrongLengthError);
        }
        // VerifyingKey uses TryFrom<&[u8]> for deserialization
        match SlhDsaVerifyingKey::<Sha2_128s>::try_from(bytes) {
            Ok(verifying_key) => Ok(PublicKey(verifying_key)),
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
    /// Generate a random private key from a cryptographically-secure RNG.
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore,
    {
        // Generate a random SigningKey directly using the RNG
        // The slh-dsa crate expects a type that implements CryptoRng from the signature crate
        // We create an adapter that implements the required traits
        use slh_dsa::signature::rand_core::{CryptoRng as SlhCryptoRng, RngCore as SlhRngCore};

        struct RngAdapter<'a, R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore>(&'a mut R);

        impl<'a, R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore> SlhRngCore for RngAdapter<'a, R> {
            fn next_u32(&mut self) -> u32 {
                self.0.next_u32()
            }
            fn next_u64(&mut self) -> u64 {
                self.0.next_u64()
            }
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                self.0.fill_bytes(dest)
            }
        }

        impl<'a, R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore> SlhCryptoRng for RngAdapter<'a, R> {}

        let mut adapter = RngAdapter(rng);
        let signing_key = SlhDsaSigningKey::<Sha2_128s>::new(&mut adapter);
        PrivateKey(signing_key)
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
        PrivateKey::from_bytes_unchecked(bytes)
    }
}

impl Length for PrivateKey {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl ValidCryptoMaterial for PrivateKey {
    const AIP_80_PREFIX: &'static str = "slh-dsa-sha2-128s-priv-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

//////////////////////
// PublicKey Traits //
//////////////////////

// Implementing From<&PrivateKey<...>> allows to derive a public key in a more elegant fashion
impl From<&PrivateKey> for PublicKey {
    fn from(private_key: &PrivateKey) -> Self {
        // SigningKey contains the public key internally
        // We can get it by signing a dummy message and extracting the public key from the signature context
        // However, a simpler approach is to use the public key bytes directly from the signing key
        // The SigningKey structure contains a VerifyingKey that we can access
        // For SLH-DSA, the public key is stored as part of the signing key structure
        // We'll extract it by getting the public key bytes
        // SigningKey implements AsRef<VerifyingKey<P>>, so we can get a reference and clone it
        let verifying_key = private_key.0.as_ref().clone();
        PublicKey(verifying_key)
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
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slh_dsa::PublicKey({})", self)
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
    const AIP_80_PREFIX: &'static str = "slh-dsa-sha2-128s-pub-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

/////////////
// Fuzzing //
/////////////

/// Produces a uniformly random SLH-DSA SHA2-128s keypair from a seed
#[cfg(any(test, feature = "fuzzing"))]
pub fn keypair_strategy() -> impl proptest::strategy::Strategy<Value = KeyPair<PrivateKey, PublicKey>> {
    test_utils::uniform_keypair_strategy::<PrivateKey, PublicKey>()
}

/// Produces a uniformly random SLH-DSA SHA2-128s public key
#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for PublicKey {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        crate::test_utils::uniform_keypair_strategy::<PrivateKey, PublicKey>()
            .prop_map(|v| v.public_key)
            .boxed()
    }
}

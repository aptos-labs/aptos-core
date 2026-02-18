// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This file implements traits for SLH-DSA-SHA2-128s private keys and public keys.

#[cfg(any(test, feature = "fuzzing"))]
use crate::test_utils::{self, KeyPair};
use crate::{
    hash::CryptoHash,
    slh_dsa_sha2_128s::{Signature, PRIVATE_KEY_LENGTH, PUBLIC_KEY_LENGTH},
    traits::{PrivateKey as PrivateKeyTrait, PublicKey as PublicKeyTrait, *},
};
use aptos_crypto_derive::{key_name, DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use core::convert::TryFrom;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
use serde::Serialize;
use slh_dsa::{Sha2_128s, SigningKey as SlhDsaSigningKey, VerifyingKey as SlhDsaVerifyingKey};
use std::fmt;

/// A SLH-DSA-SHA2-128s private key (signing key)
#[derive(DeserializeKey, SerializeKey, SilentDebug, SilentDisplay, PartialEq, Eq)]
#[key_name("SlhDsa_Sha2_128s_PrivateKey")]
pub struct PrivateKey(pub(crate) SlhDsaSigningKey<Sha2_128s>);

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(PrivateKey: Clone);

#[cfg(any(test, feature = "cloneable-private-keys"))]
impl Clone for PrivateKey {
    fn clone(&self) -> Self {
        // More efficient than our deserialization, which would slowly recompute the PK root.
        let sk_bytes: &[u8] = &(self.0.to_bytes());
        let signing_key = SlhDsaSigningKey::<Sha2_128s>::try_from(sk_bytes).unwrap();
        PrivateKey(signing_key)
    }
}

/// A SLH-DSA-SHA2-128s public key (verifying key)
#[derive(DeserializeKey, Clone, SerializeKey, PartialEq, Eq)]
#[key_name("SlhDsa_Sha2_128s_PublicKey")]
pub struct PublicKey(pub(crate) SlhDsaVerifyingKey<Sha2_128s>);

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

    /// Serialize a PrivateKey
    /// Returns only the first PRIVATE_KEY_LENGTH bytes (48 bytes), which contain
    /// the SK seed, PRF seed, and PK seed. The PK root is excluded as it's part
    /// of the public key material.
    pub fn to_bytes(&self) -> Vec<u8> {
        let full_bytes = self.0.to_bytes();
        // Extract only the first PRIVATE_KEY_LENGTH bytes (the three 16-byte seeds)
        // The full serialization includes the PK root, which we exclude
        full_bytes[..PRIVATE_KEY_LENGTH].to_vec()
    }

    /// Deserialize a PrivateKey: there are no validation checks beyond length checks.
    /// The input bytes should be PRIVATE_KEY_LENGTH (48 bytes): three 16-byte seeds.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<PrivateKey, CryptoMaterialError> {
        if bytes.len() != PRIVATE_KEY_LENGTH {
            return Err(CryptoMaterialError::WrongLengthError);
        }
        // SLH-DSA private key generation requires sk_seed, sk_prf, and pk_seed (each 16 bytes)
        // Split the 48-byte input into three 16-byte seeds
        let sk_seed: [u8; 16] = bytes[0..16]
            .try_into()
            .map_err(|_| CryptoMaterialError::WrongLengthError)?;
        let sk_prf: [u8; 16] = bytes[16..32]
            .try_into()
            .map_err(|_| CryptoMaterialError::WrongLengthError)?;
        let pk_seed: [u8; 16] = bytes[32..48]
            .try_into()
            .map_err(|_| CryptoMaterialError::WrongLengthError)?;

        let signing_key =
            SlhDsaSigningKey::<Sha2_128s>::slh_keygen_internal(&sk_seed, &sk_prf, &pk_seed);

        Ok(PrivateKey(signing_key))
    }

    /// Private function aimed at minimizing code duplication between sign
    /// methods of the SigningKey implementation. This should remain private.
    fn sign_arbitrary_message(&self, message: &[u8]) -> Signature {
        use slh_dsa::signature::Signer;
        // NOTE: To hedge against fault attacks, can use RandomizedSigner::<slh_dsa::Signature<Sha2_128s>>::sign_with_rng().
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
        use slh_dsa::signature::rand_core::{TryCryptoRng as SlhTryCryptoRng, TryRng as SlhTryRng};

        struct RngAdapter<
            'a,
            R: ::rand::RngCore + ::rand::CryptoRng + ::rand_core::CryptoRng + ::rand_core::RngCore,
        >(&'a mut R);

        impl<'a, R: ::rand::RngCore + ::rand::CryptoRng> SlhTryRng for RngAdapter<'a, R> {
            type Error = core::convert::Infallible;

            fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
                Ok(self.0.next_u32())
            }

            fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
                Ok(self.0.next_u64())
            }

            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
                self.0.fill_bytes(dest);
                Ok(())
            }
        }

        impl<'a, R: ::rand::RngCore + ::rand::CryptoRng> SlhTryCryptoRng for RngAdapter<'a, R> {}

        let mut adapter = RngAdapter(rng);
        let signing_key = SlhDsaSigningKey::<Sha2_128s>::new(&mut adapter);
        PrivateKey(signing_key)
    }
}

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
        // The SigningKey structure contains the public key (i.e., a `VerifyingKey`) that we can access
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
        write!(f, "slh_dsa_sha2_128s::PublicKey({})", self)
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

/// Produces a uniformly random SLH-DSA-SHA2-128s keypair from a seed
#[cfg(any(test, feature = "fuzzing"))]
pub fn keypair_strategy(
) -> impl proptest::strategy::Strategy<Value = KeyPair<PrivateKey, PublicKey>> {
    test_utils::uniform_keypair_strategy::<PrivateKey, PublicKey>()
}

/// Produces a uniformly random SLH-DSA-SHA2-128s public key
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{Signature as SignatureTrait, Uniform};

    #[test]
    fn test_private_key_serialization_wrong_length() {
        // Test that deserializing with wrong length fails
        let too_short = [0u8; PRIVATE_KEY_LENGTH - 1];
        assert!(
            PrivateKey::try_from(&too_short[..]).is_err(),
            "Should reject keys that are too short"
        );

        let too_long = [0u8; PRIVATE_KEY_LENGTH + 1];
        assert!(
            PrivateKey::try_from(&too_long[..]).is_err(),
            "Should reject keys that are too long"
        );
    }

    #[test]
    fn test_private_key_serialization_different_keys() {
        // Test that different bytes produce different keys
        let sk_bytes_1 = [0x01u8; PRIVATE_KEY_LENGTH];
        let sk_bytes_2 = [0x02u8; PRIVATE_KEY_LENGTH];

        let key1 =
            PrivateKey::try_from(&sk_bytes_1[..]).expect("Should create key from sk_bytes_1");
        let key2 =
            PrivateKey::try_from(&sk_bytes_2[..]).expect("Should create key from sk_bytes_2");

        // Get public keys
        let pubkey1: PublicKey = (&key1).into();
        let pubkey2: PublicKey = (&key2).into();

        // Different bytes should produce different public keys
        assert_ne!(
            pubkey1.to_bytes(),
            pubkey2.to_bytes(),
            "Different seed bytes should produce different public keys"
        );
    }

    #[test]
    fn test_private_key_generate_and_use() {
        // Test that generated keys can be used for signing and verification
        let mut rng = rand::thread_rng();
        let key = PrivateKey::generate(&mut rng);

        let pubkey: PublicKey = (&key).into();
        let test_message = b"test message";

        // Sign the message
        let signature = key.sign_arbitrary_message(test_message);

        // Verify the signature
        assert!(
            signature
                .verify_arbitrary_msg(test_message, &pubkey)
                .is_ok(),
            "Generated key should produce valid signatures"
        );

        // Verify wrong message fails
        assert!(
            signature
                .verify_arbitrary_msg(b"wrong message", &pubkey)
                .is_err(),
            "Signature should not verify for wrong message"
        );
    }

    #[test]
    fn test_private_key_uniform_generation_different() {
        // Test that two randomly generated private keys via Uniform trait are different
        let mut rng = rand::thread_rng();
        let key1 = PrivateKey::generate(&mut rng);
        let key2 = PrivateKey::generate(&mut rng);

        // Ensure the two keys are different
        assert_ne!(
            key1, key2,
            "Two randomly generated private keys should be different"
        );

        // Also verify their serialized bytes are different
        assert_ne!(
            key1.to_bytes(),
            key2.to_bytes(),
            "Serialized bytes of two randomly generated keys should be different"
        );

        // Verify their public keys are also different
        let pubkey1: PublicKey = (&key1).into();
        let pubkey2: PublicKey = (&key2).into();
        assert_ne!(
            pubkey1, pubkey2,
            "Public keys derived from two different private keys should be different"
        );
    }

    #[test]
    fn test_private_key_serialization_round_trip() {
        // Create a random key
        // Test that generated keys can be used for signing and verification
        let mut rng = rand::thread_rng();
        let original_sk = PrivateKey::generate(&mut rng);

        // Serialize the key
        let sk_bytes = original_sk.to_bytes();

        // Verify the sk_bytes length is PRIVATE_KEY_LENGTH
        assert_eq!(
            sk_bytes.len(),
            PRIVATE_KEY_LENGTH,
            "Serialized key should be exactly PRIVATE_KEY_LENGTH bytes"
        );

        // Deserialize the key
        let deserialized_sk = PrivateKey::try_from(&sk_bytes[..])
            .expect("Should be able to deserialize key from sk_bytes bytes");

        // Verify keys are the same and produce the same public key
        assert_eq!(
            original_sk.to_bytes(),
            deserialized_sk.to_bytes(),
            "Keys should be the same"
        );

        let original_pubkey: PublicKey = (&original_sk).into();
        let deserialized_pubkey: PublicKey = (&deserialized_sk).into();
        assert_eq!(
            original_pubkey.to_bytes(),
            deserialized_pubkey.to_bytes(),
            "Deserialized key should produce the same public key"
        );
    }

    #[test]
    fn test_public_key_serialization_round_trip() {
        // Pick a random SK
        let mut rng = rand::thread_rng();
        let sk = PrivateKey::generate(&mut rng);

        // Get its PK
        let original_pk: PublicKey = (&sk).into();

        // Serialize the PK
        let pk_bytes = original_pk.to_bytes();

        // Verify the pk_bytes length is PUBLIC_KEY_LENGTH
        assert_eq!(
            pk_bytes.len(),
            PUBLIC_KEY_LENGTH,
            "Serialized public key should be exactly PUBLIC_KEY_LENGTH bytes"
        );

        // Deserialize it back
        let deserialized_pk = PublicKey::try_from(&pk_bytes[..])
            .expect("Should be able to deserialize public key from bytes");

        assert_eq!(
            original_pk, deserialized_pk,
            "Deserialized public key should be equal to the original"
        );
    }

    #[test]
    fn test_signature_serialization_round_trip() {
        // Pick a random keypair
        let mut rng = rand::thread_rng();
        let sk = PrivateKey::generate(&mut rng);
        let pk: PublicKey = (&sk).into();

        let message = [0x42u8; 32];

        // Sign the message and verify the signature
        let original_sig = sk.sign_arbitrary_message(&message);
        assert!(
            original_sig.verify_arbitrary_msg(&message, &pk).is_ok(),
            "Original signature should verify correctly"
        );

        // Serialize the signature
        let sig_bytes = original_sig.to_bytes();

        // Verify the sig_bytes length is SIGNATURE_LENGTH
        assert_eq!(
            sig_bytes.len(),
            Signature::LENGTH,
            "Serialized signature should be exactly SIGNATURE_LENGTH bytes"
        );

        // Deserialize it back
        let deserialized_sig = Signature::try_from(&sig_bytes[..])
            .expect("Should be able to deserialize signature from bytes");

        // assert_eq the two signatures
        assert_eq!(
            original_sig, deserialized_sig,
            "Deserialized signature should be equal to the original"
        );

        // Also verify the deserialized signature still verifies
        assert!(
            deserialized_sig.verify_arbitrary_msg(&message, &pk).is_ok(),
            "Deserialized signature should still verify correctly"
        );
    }

    #[test]
    fn test_private_key_clone() {
        // Generate a random private key
        let mut rng = rand::thread_rng();
        let original_key = PrivateKey::generate(&mut rng);

        // Clone the private key
        let cloned_key = original_key.clone();

        // Assert the cloned key is equal to the original
        assert_eq!(
            original_key, cloned_key,
            "Cloned private key should be equal to the original"
        );
    }

    #[test]
    fn test_signing_is_deterministic() {
        // Generate a random private key
        let mut rng = rand::thread_rng();
        let key = PrivateKey::generate(&mut rng);

        // Create a test message
        let message = b"test message for deterministic signing";

        // Sign the same message twice
        let signature1 = key.sign_arbitrary_message(message);
        let signature2 = key.sign_arbitrary_message(message);

        // Assert that the two signatures are identical
        assert_eq!(
            signature1, signature2,
            "Signing the same message twice should produce identical signatures"
        );

        // Also verify the signatures are equal when comparing bytes
        assert_eq!(
            signature1.to_bytes(),
            signature2.to_bytes(),
            "Signature bytes should be identical for the same message"
        );
    }
}

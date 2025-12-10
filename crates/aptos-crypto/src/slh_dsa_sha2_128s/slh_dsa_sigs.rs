// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This file implements traits for SLH-DSA SHA2-128s signatures.

use super::SIGNATURE_LENGTH;
use crate::{
    hash::CryptoHash,
    slh_dsa_sha2_128s::{PrivateKey, PublicKey},
    traits::{Signature as SignatureTrait, *},
};
use anyhow::{anyhow, Result};
use aptos_crypto_derive::{key_name, DeserializeKey, SerializeKey};
use core::convert::TryFrom;
use serde::Serialize;
use slh_dsa::{Sha2_128s, Signature as SlhDsaSignature};
use std::fmt;

/// A SLH-DSA SHA2-128s signature.
/// NOTE: The max size on this struct is enforced in its `TryFrom<u8>` trait implementation.
#[derive(DeserializeKey, Clone, SerializeKey, PartialEq, Eq)]
#[key_name("SlhDsa_Sha2_128s_Signature")]
pub struct Signature(pub(crate) SlhDsaSignature<Sha2_128s>);

impl Signature {
    /// The length of the Signature
    pub const LENGTH: usize = SIGNATURE_LENGTH;

    /// Serialize a Signature
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }

    /// Deserialize a Signature, without validation
    pub fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<Signature, CryptoMaterialError> {
        if bytes.len() != SIGNATURE_LENGTH {
            return Err(CryptoMaterialError::WrongLengthError);
        }
        // Signature uses TryFrom<&[u8]> for deserialization
        match SlhDsaSignature::<Sha2_128s>::try_from(bytes) {
            Ok(signature) => Ok(Signature(signature)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }

    /// return an all-zero signature (for test only)
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_signature() -> Self {
        // Create a dummy signature with ones - this is only for testing
        let bytes = vec![1u8; SIGNATURE_LENGTH];
        Self::from_bytes_unchecked(&bytes).unwrap()
    }
}

//////////////////////
// Signature Traits //
//////////////////////

impl SignatureTrait for Signature {
    type SigningKeyMaterial = PrivateKey;
    type VerifyingKeyMaterial = PublicKey;

    /// Verifies that the provided signature is valid for the provided message.
    fn verify<T: CryptoHash + Serialize>(&self, message: &T, public_key: &PublicKey) -> Result<()> {
        Self::verify_arbitrary_msg(self, &signing_message(message)?, public_key)
    }

    /// Checks that `self` is valid for an arbitrary &[u8] `message` using `public_key`.
    /// Outside of this crate, this particular function should only be used for native signature
    /// verification in Move.
    fn verify_arbitrary_msg(&self, message: &[u8], public_key: &PublicKey) -> Result<()> {
        use slh_dsa::signature::Verifier;
        Verifier::<SlhDsaSignature<Sha2_128s>>::verify(&public_key.0, message, &self.0)
            .map_err(|e| anyhow!("SLH-DSA signature verification failed: {}", e))
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl Length for Signature {
    fn length(&self) -> usize {
        SIGNATURE_LENGTH
    }
}

impl ValidCryptoMaterial for Signature {
    const AIP_80_PREFIX: &'static str = "slh-dsa-sha2-128s-sig-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl std::hash::Hash for Signature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_signature = self.to_bytes();
        state.write(&encoded_signature);
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<Signature, CryptoMaterialError> {
        Signature::from_bytes_unchecked(bytes)
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.to_bytes()[..]))
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slh_dsa_sha2_128s::Signature({})", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_signature_deserializes() {
        // Create a dummy signature by deserializing some dummy bytes.
        // This test simply ensures this doesn't panic.
        let _ = Signature::dummy_signature();
    }
}

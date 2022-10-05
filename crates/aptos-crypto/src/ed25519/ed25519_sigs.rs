// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file implements traits for Ed25519 signatures.

use crate::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey, ED25519_SIGNATURE_LENGTH, L},
    hash::CryptoHash,
    traits::*,
};
use anyhow::{anyhow, Result};
use aptos_crypto_derive::{DeserializeKey, SerializeKey};
use core::convert::TryFrom;
use serde::Serialize;
use std::{cmp::Ordering, fmt};

/// An Ed25519 signature
#[derive(DeserializeKey, Clone, SerializeKey)]
pub struct Ed25519Signature(pub(crate) ed25519_dalek::Signature);

impl Ed25519Signature {
    /// The length of the Ed25519Signature
    pub const LENGTH: usize = ed25519_dalek::SIGNATURE_LENGTH;

    /// Serialize an Ed25519Signature.
    pub fn to_bytes(&self) -> [u8; ED25519_SIGNATURE_LENGTH] {
        self.0.to_bytes()
    }

    /// Deserialize an Ed25519Signature without any validation checks (malleability)
    /// apart from expected signature size.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<Ed25519Signature, CryptoMaterialError> {
        match ed25519_dalek::Signature::try_from(bytes) {
            Ok(dalek_signature) => Ok(Ed25519Signature(dalek_signature)),
            Err(_) => Err(CryptoMaterialError::DeserializationError),
        }
    }

    /// return an all-zero signature (for test only)
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_signature() -> Self {
        Self::from_bytes_unchecked(&[0u8; Self::LENGTH]).unwrap()
    }

    /// Check for correct size and third-party based signature malleability issues.
    /// This method is required to ensure that given a valid signature for some message under some
    /// key, an attacker cannot produce another valid signature for the same message and key.
    ///
    /// According to [RFC8032](https://tools.ietf.org/html/rfc8032), signatures comprise elements
    /// {R, S} and we should enforce that S is of canonical form (smaller than L, where L is the
    /// order of edwards25519 curve group) to prevent signature malleability. Without this check,
    /// one could add a multiple of L into S and still pass signature verification, resulting in
    /// a distinct yet valid signature.
    ///
    /// This method does not check the R component of the signature, because R is hashed during
    /// signing and verification to compute h = H(ENC(R) || ENC(A) || M), which means that a
    /// third-party cannot modify R without being detected.
    ///
    /// Note: It's true that malicious signers can already produce varying signatures by
    /// choosing a different nonce, so this method protects against malleability attacks performed
    /// by a non-signer.
    pub fn check_s_malleability(bytes: &[u8]) -> std::result::Result<(), CryptoMaterialError> {
        if bytes.len() != ED25519_SIGNATURE_LENGTH {
            return Err(CryptoMaterialError::WrongLengthError);
        }
        if !Ed25519Signature::check_s_lt_l(&bytes[32..]) {
            return Err(CryptoMaterialError::CanonicalRepresentationError);
        }
        Ok(())
    }

    /// Check if S < L to capture invalid signatures.
    fn check_s_lt_l(s: &[u8]) -> bool {
        for i in (0..32).rev() {
            match s[i].cmp(&L[i]) {
                Ordering::Less => return true,
                Ordering::Greater => return false,
                _ => {}
            }
        }
        // As this stage S == L which implies a non canonical S.
        false
    }
}

//////////////////////
// Signature Traits //
//////////////////////

impl Signature for Ed25519Signature {
    type VerifyingKeyMaterial = Ed25519PublicKey;
    type SigningKeyMaterial = Ed25519PrivateKey;

    /// Verifies that the provided signature is valid for the provided message, going beyond the
    /// [RFC8032](https://tools.ietf.org/html/rfc8032) specification, checking both scalar
    /// malleability and point malleability (see documentation [here](https://docs.rs/ed25519-dalek/latest/ed25519_dalek/struct.PublicKey.html#on-the-multiple-sources-of-malleability-in-ed25519-signatures)).
    ///
    /// This _strict_ verification performs steps 1,2 and 3 from Section 5.1.7 in RFC8032, and an
    /// additional scalar malleability check (via [Ed25519Signature::check_s_malleability][Ed25519Signature::check_s_malleability]).
    ///
    /// This function will ensure both the signature and the `public_key` are not in a small subgroup.
    fn verify<T: CryptoHash + Serialize>(
        &self,
        message: &T,
        public_key: &Ed25519PublicKey,
    ) -> Result<()> {
        Self::verify_arbitrary_msg(self, &signing_message(message)?, public_key)
    }

    /// Checks that `self` is valid for an arbitrary &[u8] `message` using `public_key`.
    /// Outside of this crate, this particular function should only be used for native signature
    /// verification in Move.
    ///
    /// This function will check both the signature and `public_key` for small subgroup attacks.
    fn verify_arbitrary_msg(&self, message: &[u8], public_key: &Ed25519PublicKey) -> Result<()> {
        // NOTE: ed25519::PublicKey::verify_strict already checks that the s-component of the signature
        // is not mauled, but does so via an optimistic path which fails into a slower path. By doing
        // our own (much faster) checking here, we can ensure dalek's optimistic path always succeeds
        // and the slow path is never triggered.
        Ed25519Signature::check_s_malleability(&self.to_bytes())?;

        // NOTE: ed25519::PublicKey::verify_strict checks that the signature's R-component and
        // the public key are *not* in a small subgroup.
        public_key
            .0
            .verify_strict(message, &self.0)
            .map_err(|e| anyhow!("{}", e))
            .and(Ok(()))
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }
}

impl Length for Ed25519Signature {
    fn length(&self) -> usize {
        ED25519_SIGNATURE_LENGTH
    }
}

impl ValidCryptoMaterial for Ed25519Signature {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl std::hash::Hash for Ed25519Signature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_signature = self.to_bytes();
        state.write(&encoded_signature);
    }
}

impl TryFrom<&[u8]> for Ed25519Signature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<Ed25519Signature, CryptoMaterialError> {
        // We leave this check here to detect mauled signatures earlier, since it does not hurt
        // performance much. (This check is performed again in Ed25519Signature::verify_arbitrary_msg
        // and in ed25519-dalek's verify_strict API.)
        Ed25519Signature::check_s_malleability(bytes)?;
        Ed25519Signature::from_bytes_unchecked(bytes)
    }
}

// Those are required by the implementation of hash above
impl PartialEq for Ed25519Signature {
    fn eq(&self, other: &Ed25519Signature) -> bool {
        self.to_bytes()[..] == other.to_bytes()[..]
    }
}

impl Eq for Ed25519Signature {}

impl fmt::Display for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0.to_bytes()[..]))
    }
}

impl fmt::Debug for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519Signature({})", self)
    }
}

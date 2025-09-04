// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file implements traits for ECDSA signatures over Secp256r1.

use super::SIGNATURE_LENGTH;
use crate::{
    hash::CryptoHash,
    secp256r1_ecdsa::{PrivateKey, PublicKey, ORDER_HALF},
    traits::{Signature as SignatureTrait, *},
};
use anyhow::{anyhow, Result};
use velor_crypto_derive::{key_name, DeserializeKey, SerializeKey};
use core::convert::TryFrom;
use p256::NonZeroScalar;
use serde::Serialize;
use signature::Verifier;
use std::{cmp::Ordering, fmt};

/// A secp256r1 ECDSA signature.
/// NOTE: The max size on this struct is enforced in its `TryFrom<u8>` trait implementation.
#[derive(DeserializeKey, Clone, SerializeKey)]
#[key_name("Secp256r1EcdsaSignature")]
pub struct Signature(pub(crate) p256::ecdsa::Signature);

impl Signature {
    /// The length of the Signature
    pub const LENGTH: usize = SIGNATURE_LENGTH;

    /// Serialize an Signature. Uses the SEC1 serialization format.
    pub fn to_bytes(&self) -> [u8; SIGNATURE_LENGTH] {
        // The RustCrypto P256 `to_bytes` call here should never return a byte array of the wrong length
        self.0.to_bytes().into()
    }

    /// Deserialize an P256Signature, without checking for malleability
    /// Uses the SEC1 serialization format.
    pub fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<Signature, CryptoMaterialError> {
        match p256::ecdsa::Signature::try_from(bytes) {
            Ok(p256_signature) => Ok(Signature(p256_signature)),
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
    /// We use the technique described in
    /// [BIP146](https://github.com/bitcoin/bips/blob/master/bip-0146.mediawiki) to prevent
    /// malleability of ECDSA signatures. Signatures comprise elements {R, S}, and S can be
    /// enforced to be of canonical form by ensuring it is less than the order of the Secp256r1 curve
    /// divided by 2. If this is not done, a value S > n/2 can be replaced by S' = n - S to form another distinct valid
    /// signature, where n is the curve order. This check is not performed by the RustCrypto P256 library
    /// we use
    pub fn check_s_malleability(bytes: &[u8]) -> std::result::Result<(), CryptoMaterialError> {
        if bytes.len() != SIGNATURE_LENGTH {
            return Err(CryptoMaterialError::WrongLengthError);
        }
        if !Signature::check_s_lt_order_half(&bytes[32..]) {
            return Err(CryptoMaterialError::CanonicalRepresentationError);
        }
        Ok(())
    }

    /// Check if S < ORDER_HALF to capture invalid signatures.
    fn check_s_lt_order_half(s: &[u8]) -> bool {
        for i in 0..32 {
            match s[i].cmp(&ORDER_HALF[i]) {
                Ordering::Less => return true,
                Ordering::Greater => return false,
                _ => {},
            }
        }
        // At this stage S == ORDER_HALF which implies a non canonical S.
        false
    }

    /// If the signature {R,S} does not have S < n/2 where n is the Ristretto255 order, return
    /// {R,n-S} as the canonical encoding of this signature to prevent malleability attacks. See
    /// `check_s_malleability` for more detail
    pub fn make_canonical(&self) -> Signature {
        if Signature::check_s_malleability(&self.to_bytes()[..]).is_ok() {
            return self.clone();
        };
        let s = self.0.s();
        let r = self.0.r();
        let new_s = -*s;
        let new_s_nonzero = NonZeroScalar::new(new_s).unwrap();
        let new_sig = p256::ecdsa::Signature::from_scalars(r, new_s_nonzero).unwrap();
        Signature(new_sig)
    }

    /// If signature bytes are serialized correctly, this function will return a canonical signature
    /// that passes malleability checks.
    #[cfg(feature = "testing")]
    pub fn make_canonical_from_bytes_unchecked(
        bytes: &[u8],
    ) -> Result<Signature, CryptoMaterialError> {
        let signature = Signature::from_bytes_unchecked(bytes)?;
        Ok(Signature::make_canonical(&signature))
    }
}

//////////////////////
// Signature Traits //
//////////////////////

impl SignatureTrait for Signature {
    type SigningKeyMaterial = PrivateKey;
    type VerifyingKeyMaterial = PublicKey;

    /// Verifies that the provided signature is valid for the provided message, going beyond the
    /// [NIST SP 800-186](https://csrc.nist.gov/publications/detail/sp/800-186/final) specification, to prevent scalar malleability as done in [BIP146](https://github.com/bitcoin/bips/blob/master/bip-0146.mediawiki).
    fn verify<T: CryptoHash + Serialize>(&self, message: &T, public_key: &PublicKey) -> Result<()> {
        Self::verify_arbitrary_msg(self, &signing_message(message)?, public_key)
    }

    /// Checks that `self` is valid for an arbitrary &[u8] `message` using `public_key`.
    /// Outside of this crate, this particular function should only be used for native signature
    /// verification in Move.
    ///
    /// Checks for and rejects non-canonical signatures (r,s) where s > (n/2), where n is the group
    /// order
    fn verify_arbitrary_msg(&self, message: &[u8], public_key: &PublicKey) -> Result<()> {
        Signature::check_s_malleability(&self.to_bytes())?;

        public_key
            .0
            .verify(message, &self.0)
            .map_err(|e| anyhow!("{}", e))
            .and(Ok(()))
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }
}

impl Length for Signature {
    fn length(&self) -> usize {
        SIGNATURE_LENGTH
    }
}

impl ValidCryptoMaterial for Signature {
    const AIP_80_PREFIX: &'static str = "secp256r1-sig-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
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
        Signature::check_s_malleability(bytes)?;
        Signature::from_bytes_unchecked(bytes)
    }
}

// Those are required by the implementation of hash above
impl PartialEq for Signature {
    fn eq(&self, other: &Signature) -> bool {
        self.to_bytes()[..] == other.to_bytes()[..]
    }
}

impl Eq for Signature {}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0.to_bytes()[..]))
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "secp256r1_ecdsa::Signature({})", self)
    }
}

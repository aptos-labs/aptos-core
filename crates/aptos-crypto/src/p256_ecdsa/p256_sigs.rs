// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file implements traits for ECDSA signatures over NIST-P256.

use super::P256_SIGNATURE_LENGTH;
use crate::{
    hash::CryptoHash,
    p256_ecdsa::{P256PrivateKey, P256PublicKey, ORDER_HALF},
    traits::*,
};
use anyhow::{anyhow, Result};
use aptos_crypto_derive::{DeserializeKey, SerializeKey};
use core::convert::TryFrom;
use p256::NonZeroScalar;
use serde::Serialize;
use signature::Verifier;
use std::{cmp::Ordering, fmt};

/// A P256 signature
#[derive(DeserializeKey, Clone, SerializeKey)]
pub struct P256Signature(pub(crate) p256::ecdsa::Signature);

impl P256Signature {
    /// The length of the P256Signature
    pub const LENGTH: usize = P256_SIGNATURE_LENGTH;

    /// Serialize an P256Signature. Uses the SEC1 serialization format.
    pub fn to_bytes(&self) -> [u8; P256_SIGNATURE_LENGTH] {
        // The RustCrypto P256 `to_bytes` call here should never return a byte array of the wrong length
        self.0.to_bytes().try_into().unwrap()
    }

    /// Deserialize an P256Signature, without checking for malleability
    /// Uses the SEC1 serialization format.
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<P256Signature, CryptoMaterialError> {
        match p256::ecdsa::Signature::try_from(bytes) {
            Ok(p256_signature) => Ok(P256Signature(p256_signature)),
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
    /// enforced to be of canonical form by ensuring it is less than the order of the P256 curve
    /// divided by 2. If this is not done, a value S > n/2 can be replaced by S' = n - S to form another distinct valid
    /// signature, where n is the curve order. This check is not performed by the RustCrypto P256 library
    /// we use
    pub fn check_s_malleability(bytes: &[u8]) -> std::result::Result<(), CryptoMaterialError> {
        if bytes.len() != P256_SIGNATURE_LENGTH {
            return Err(CryptoMaterialError::WrongLengthError);
        }
        if !P256Signature::check_s_lt_order_half(&bytes[32..]) {
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
    pub fn make_canonical(&self) -> P256Signature {
        if P256Signature::check_s_malleability(&self.to_bytes()[..]).is_ok() {
            return self.clone();
        };
        let s = self.0.s();
        let r = self.0.r();
        let new_s = -*s;
        let new_s_nonzero = NonZeroScalar::new(new_s).unwrap();
        let new_sig = p256::ecdsa::Signature::from_scalars(r, new_s_nonzero).unwrap();
        P256Signature(new_sig)
    }
}

//////////////////////
// Signature Traits //
//////////////////////

impl Signature for P256Signature {
    type SigningKeyMaterial = P256PrivateKey;
    type VerifyingKeyMaterial = P256PublicKey;

    /// Verifies that the provided signature is valid for the provided message, going beyond the
    /// [NIST SP 800-186](https://csrc.nist.gov/publications/detail/sp/800-186/final) specification, to prevent scalar malleability as done in [BIP146](https://github.com/bitcoin/bips/blob/master/bip-0146.mediawiki).
    fn verify<T: CryptoHash + Serialize>(
        &self,
        message: &T,
        public_key: &P256PublicKey,
    ) -> Result<()> {
        Self::verify_arbitrary_msg(self, &signing_message(message)?, public_key)
    }

    /// Checks that `self` is valid for an arbitrary &[u8] `message` using `public_key`.
    /// Outside of this crate, this particular function should only be used for native signature
    /// verification in Move.
    ///
    /// Checks for and rejects non-canonical signatures (r,s) where s > (n/2), where n is the group
    /// order
    fn verify_arbitrary_msg(&self, message: &[u8], public_key: &P256PublicKey) -> Result<()> {
        P256Signature::check_s_malleability(&self.to_bytes())?;

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

impl Length for P256Signature {
    fn length(&self) -> usize {
        P256_SIGNATURE_LENGTH
    }
}

impl ValidCryptoMaterial for P256Signature {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl std::hash::Hash for P256Signature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_signature = self.to_bytes();
        state.write(&encoded_signature);
    }
}

impl TryFrom<&[u8]> for P256Signature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<P256Signature, CryptoMaterialError> {
        P256Signature::check_s_malleability(bytes)?;
        P256Signature::from_bytes_unchecked(bytes)
    }
}

// Those are required by the implementation of hash above
impl PartialEq for P256Signature {
    fn eq(&self, other: &P256Signature) -> bool {
        self.to_bytes()[..] == other.to_bytes()[..]
    }
}

impl Eq for P256Signature {}

impl fmt::Display for P256Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0.to_bytes()[..]))
    }
}

impl fmt::Debug for P256Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P256Signature({})", self)
    }
}

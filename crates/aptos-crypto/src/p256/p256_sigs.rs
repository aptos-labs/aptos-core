// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file implements traits for P256 signatures.

use crate::{
    p256::{P256PrivateKey, P256PublicKey},
    hash::CryptoHash,
    traits::*,
};
use anyhow::{anyhow, Result};
use aptos_crypto_derive::{DeserializeKey, SerializeKey};
use core::convert::TryFrom;
use serde::Serialize;
use std::{cmp::Ordering, fmt};

use super::{P256_SIGNATURE_LENGTH, ORDER_HALF};
use openssl::ecdsa::EcdsaSig;

/// A P256 signature
#[derive(DeserializeKey, SerializeKey)]
pub struct P256Signature(pub(crate) EcdsaSig);

impl Clone for P256Signature {
    fn clone(&self) -> Self {
        let serialized: &[u8] = &(self.to_bytes());
        P256Signature::try_from(serialized).unwrap()
    }
}

impl private::Sealed for P256Signature {}

impl P256Signature {
    /// The length of the P256Signature
    pub const LENGTH: usize = P256_SIGNATURE_LENGTH;

    /// Serialize a P256Signature.
    pub fn to_bytes(&self) -> [u8; P256_SIGNATURE_LENGTH] {
        let bytes = self.0.to_der().expect("openssl ffi failed to serialize P256 signature"); 
        bytes.try_into().expect("openssl ffi serialized P256 signature incorrectly")
    }

    /// Deserialize a P256Signature without any validation checks (malleability)
    /// apart from expected signature size.
    // TODO: Check malleability here? Make "checked"
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> std::result::Result<P256Signature, CryptoMaterialError> {
        match EcdsaSig::from_der(bytes) {
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
    /// signature, where n is the curve order. This check is not performed by the openssl library
    /// we use
    pub fn check_s_malleability(bytes: &[u8]) -> std::result::Result<(), CryptoMaterialError> {
        if bytes.len() != P256_SIGNATURE_LENGTH {
            return Err(CryptoMaterialError::WrongLengthError);
        }
        // TODO: make sure this is s
        if !P256Signature::check_s_lt_order_half(&bytes[32..]) {
            return Err(CryptoMaterialError::CanonicalRepresentationError);
        }
        Ok(())
    }

    /// Check if S < ORDER_HALF to capture invalid signatures.
    fn check_s_lt_order_half(s: &[u8]) -> bool {
        for i in (0..32).rev() {
            match s[i].cmp(&ORDER_HALF[i]) {
                Ordering::Less => return true,
                Ordering::Greater => return false,
                _ => {},
            }
        }
        // As this stage S == L which implies a non canonical S.
        false
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
    /// This function will check both the signature and `public_key` for small subgroup attacks.
    fn verify_arbitrary_msg(&self, message: &[u8], public_key: &P256PublicKey) -> Result<()> {
        P256Signature::check_s_malleability(&self.to_bytes())?;

        self.0.verify(message, &public_key.0).map_err(|e| anyhow!("{}", e)).and(Ok(()))
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_der().expect("openssl ffi failed to serialize P256 signature")
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
        // We leave this check here to detect mauled signatures earlier, since it does not hurt
        // performance much. (This check is performed again in P256Signature::verify_arbitrary_msg
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
        write!(f, "{}", hex::encode(&self.0.to_der().expect("openssl ffi failed to serialize P256 signature")[..]))
    }
}

impl fmt::Debug for P256Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P256Signature({})", self)
    }
}

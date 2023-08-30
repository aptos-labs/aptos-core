// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file implements traits for WebAuthn signatures over NIST-P256.

use crate::webauthn::webauthn_traits::WebAuthnSignature;
use crate::{
    hash::CryptoHash,
    p256_ecdsa::P256Signature,
    traits::*,
    webauthn::{webauthn_p256_keys::WebAuthnP256PrivateKey, WebAuthnP256PublicKey},
};
use anyhow::anyhow;
use aptos_crypto_derive::{DeserializeKey, SerializeKey};
use core::convert::TryFrom;
use serde::Serialize;
use signature::Verifier;
use std::fmt;
use webauthn_rs_core::assertion::{
    generate_verification_data, p256_der_to_fixed_size_signature, parse_bcs_encoded_paarr_vector,
};

/// A WebAuthn P256 signature
/// This is a BCS Serialized vector of bytes that contains
/// [`PartialAuthenticatorAssertionResponseRaw`](webauthn_rs_core::assertion::PartialAuthenticatorAssertionResponseRaw)
/// fields as a BCS serialized vector.
///
/// Vector items were serialized in the following order:
/// 1. `signature`
/// 2. `authenticator_data`
/// 3. `client_data_json`
///
/// See [`parse_bcs_encoded_paarr_vector`](webauthn_rs_core::assertion::parse_bcs_encoded_paarr_vector)
/// for more info on how its serialized
#[derive(DeserializeKey, Clone, SerializeKey)]
pub struct WebAuthnP256Signature(pub(crate) Vec<u8>);

impl private::Sealed for WebAuthnP256Signature {}

impl WebAuthnP256Signature {
    /// Serialize a WebAuthnP256Signature.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.as_slice().to_vec()
    }

    /// Deserialize a WebAuthnP256Signature
    pub(crate) fn from_bytes_unchecked(
        bytes: &[u8],
    ) -> Result<WebAuthnP256Signature, CryptoMaterialError> {
        Ok(WebAuthnP256Signature(bytes.to_vec()))
    }

    /// return an all-zero signature (for test only)
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_signature() -> Self {
        WebAuthnP256Signature { 0: vec![] }
    }
}

/////////////////////
// WebAuthn Traits //
/////////////////////

impl WebAuthnSignature for WebAuthnP256Signature {
    type VerifyingKeyMaterial = WebAuthnP256PublicKey;
    type SigningKeyMaterial = WebAuthnP256PrivateKey;

    /// Verifies an arbitrary challenge
    /// 1.  Decodes `PartialAuthenticatorAssertionResponse` from the bcs encoded bytes of the signature
    /// 2.  Deep equal check to see if the provided `message` (RawTransaction) matches the `actual_challenge`
    ///     stored on the `WebAuthnP256Signature`.
    /// 3.  Uses the public key to verify the signature on `verification_data`
    #[inline]
    fn verify_arbitrary_challenge(
        &self,
        message: &[u8],
        public_key: &Self::VerifyingKeyMaterial,
    ) -> anyhow::Result<()> {
        // Decode PartialAuthenticatorAssertionResponse from bytes
        let paarr = parse_bcs_encoded_paarr_vector(self.0.as_slice())?;
        // Generate P256 Signature object from signature bytes
        let p256_sig = p256_der_to_fixed_size_signature(paarr.signature.as_slice())?;

        // Check if message (RawTransaction) matches actual challenge in signature
        let is_equal = self.verify_expected_challenge_from_message_matches_actual(
            message,
            paarr.client_data.challenge.0.as_slice(),
        );

        // Check if expected challenge and actual challenge match. If there's no match, throw error
        if !is_equal {
            return Err(anyhow!(
                "Error: WebAuthn expected challenge did not match actual challenge"
            ));
        }

        // Generate verification data
        let verification_data = generate_verification_data(
            paarr.authenticator_data_bytes.as_slice(),
            paarr.client_data_bytes.as_slice(),
        );

        // Verify signature against verification data
        public_key
            .0
             .0
            .verify(verification_data.as_slice(), &p256_sig)
            .map_err(|e| anyhow!("{}", e))
            .and(Ok(()))
    }
}

//////////////////////
// Signature Traits //
//////////////////////

impl Signature for WebAuthnP256Signature {
    type VerifyingKeyMaterial = WebAuthnP256PublicKey;
    type SigningKeyMaterial = WebAuthnP256PrivateKey;

    /// Verifies that the provided signature is valid for the provided message, going beyond the
    /// [NIST SP 800-186](https://csrc.nist.gov/publications/detail/sp/800-186/final) specification,
    /// to prevent scalar malleability as done in [BIP146](https://github.com/bitcoin/bips/blob/master/bip-0146.mediawiki).
    ///
    /// NOTE: this is a feature of the underlying P256 implementation rather than the WebAuthP256 implementation
    fn verify<T: CryptoHash + Serialize>(
        &self,
        message: &T,
        public_key: &WebAuthnP256PublicKey,
    ) -> anyhow::Result<()> {
        Self::verify_arbitrary_msg(self, &signing_message(message)?, public_key)
    }

    /// Checks that `self` is valid for an arbitrary &[u8] `message` using `public_key`.
    /// Outside of this crate, this particular function should only be used for native signature
    /// verification in Move.
    fn verify_arbitrary_msg(
        &self,
        message: &[u8],
        public_key: &WebAuthnP256PublicKey,
    ) -> anyhow::Result<()> {
        self.verify_arbitrary_challenge(message, public_key)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Length for WebAuthnP256Signature {
    fn length(&self) -> usize {
        P256Signature::LENGTH
    }
}

impl ValidCryptoMaterial for WebAuthnP256Signature {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl std::hash::Hash for WebAuthnP256Signature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_signature = self.to_bytes();
        state.write(&encoded_signature);
    }
}

impl TryFrom<&[u8]> for WebAuthnP256Signature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<WebAuthnP256Signature, CryptoMaterialError> {
        WebAuthnP256Signature::from_bytes_unchecked(bytes)
    }
}

// Those are required by the implementation of hash above
impl PartialEq for WebAuthnP256Signature {
    fn eq(&self, other: &WebAuthnP256Signature) -> bool {
        self.to_bytes()[..] == other.to_bytes()[..]
    }
}

impl Eq for WebAuthnP256Signature {}

impl fmt::Display for WebAuthnP256Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.to_bytes()[..]))
    }
}

impl fmt::Debug for WebAuthnP256Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WebAuthnP256Signature({})", self)
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides traits for all WebAuthn based keys and signatures

use crate::{hash::HashValue, SigningKey, VerifyingKey};

/// WebAuthn Signature trait
pub trait WebAuthnSignature {
    /// The associated verifying key type for this signature.
    type VerifyingKeyMaterial: VerifyingKey<SignatureMaterial = Self>;
    /// The associated signing key type for this signature
    type SigningKeyMaterial: SigningKey<SignatureMaterial = Self>;

    /// WebAuthn adaptation of [`verify_arbitrary_message`](crate::traits::Signature).
    ///
    /// For WebAuthn, the `challenge` provided to `authenticatorGetAssertion` is the
    /// SHA3-256 of the `RawTransaction`.
    /// See §6.3.3 `authenticatorGetAssertion` for more info
    ///
    /// This function should do the following:
    /// 1. Verify `actual_challenge` and expected challenge from message are equal
    /// 2. Signature verification
    fn verify_arbitrary_challenge(
        &self,
        message: &[u8],
        public_key: &Self::VerifyingKeyMaterial,
    ) -> anyhow::Result<()>;

    /// This checks that the SHA3-256 of the message (`RawTransaction`)
    /// is equal to the actual challenge from the WebAuthn signature
    fn verify_expected_challenge_from_message_matches_actual(
        &self,
        message: &[u8],
        actual_challenge: &[u8],
    ) -> bool {
        // Expected challenge is SHA3-256 digest of RawTransaction bytes
        let expected_challenge = HashValue::sha3_256_of(message);

        // Compute deep equal of actual challenge and message (expected challenge)
        let deep_equal_challenge = actual_challenge
            .iter()
            .zip(expected_challenge.to_vec().iter())
            .all(|(a, b)| a == b);

        // Verify that message is equal to challenge
        deep_equal_challenge
    }
}

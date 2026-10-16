// Adapted from sigstore-rs (Apache 2.0 License)
// https://github.com/sigstore/sigstore-rs/blob/main/src/crypto/keyring.rs
//
// Copyright 2023 The Sigstore Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! CT signing key management adapted from sigstore-rs.
//!
//! This module provides a keyring for Certificate Transparency log public keys,
//! adapted to use the p256 crate instead of aws-lc-rs.

use std::collections::HashMap;

use der::{Decode, Encode};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use sha2::{Digest, Sha256};
use thiserror::Error;
use x509_cert::spki::SubjectPublicKeyInfoOwned;

/// Errors from keyring operations
#[derive(Error, Debug)]
pub enum KeyringError {
    #[error("malformed key")]
    KeyMalformed(#[from] der::Error),
    #[error("unsupported algorithm")]
    AlgoUnsupported,
    #[error("requested key not in keyring")]
    KeyNotFound,
    #[error("key not valid at requested time")]
    KeyNotValidAtTime,
    #[error("verification failed")]
    VerificationFailed,
}

type Result<T> = std::result::Result<T, KeyringError>;

/// A CT signing key.
struct Key {
    inner: VerifyingKey,
    /// The key's RFC 6962-style "key ID".
    /// <https://datatracker.ietf.org/doc/html/rfc6962#section-3.2>
    fingerprint: [u8; 32],
    /// Validity period start (Unix timestamp in seconds). None = no lower bound.
    valid_from: Option<u64>,
    /// Validity period end (Unix timestamp in seconds). None = no upper bound.
    valid_until: Option<u64>,
}

/// OID for EC public key (1.2.840.10045.2.1)
const ID_EC_PUBLIC_KEY: &str = "1.2.840.10045.2.1";
/// OID for P-256 curve (1.2.840.10045.3.1.7)
const SECP_256_R_1: &str = "1.2.840.10045.3.1.7";

impl Key {
    /// Creates a `Key` from a DER blob containing a SubjectPublicKeyInfo object.
    pub fn new(spki_bytes: &[u8]) -> Result<Self> {
        let spki = SubjectPublicKeyInfoOwned::from_der(spki_bytes)?;

        // Check algorithm parameters
        let params = spki
            .algorithm
            .parameters
            .as_ref()
            .ok_or(KeyringError::AlgoUnsupported)?;

        let algo_oid = spki.algorithm.oid.to_string();
        let params_oid = params
            .decode_as::<der::asn1::ObjectIdentifier>()
            .map_err(|_| KeyringError::AlgoUnsupported)?
            .to_string();

        // Only support EC P-256 keys
        if algo_oid != ID_EC_PUBLIC_KEY || params_oid != SECP_256_R_1 {
            return Err(KeyringError::AlgoUnsupported);
        }

        // Parse the public key
        let inner = VerifyingKey::from_sec1_bytes(spki.subject_public_key.raw_bytes())
            .map_err(|_| KeyringError::AlgoUnsupported)?;

        // Compute RFC 6962 key ID (SHA-256 hash of DER-encoded SPKI)
        let fingerprint = {
            let mut der_bytes = Vec::new();
            spki.encode_to_vec(&mut der_bytes)
                .expect("failed to hash key!");
            Sha256::digest(&der_bytes).into()
        };

        Ok(Key {
            inner,
            fingerprint,
            valid_from: None,
            valid_until: None,
        })
    }

    /// Sets the validity period for this key.
    fn with_validity(mut self, valid_from: Option<u64>, valid_until: Option<u64>) -> Self {
        self.valid_from = valid_from;
        self.valid_until = valid_until;
        self
    }
}

/// Represents a set of CT signing keys, each of which is potentially a valid signer for
/// Signed Certificate Timestamps (SCTs) or Signed Tree Heads (STHs).
///
/// Adapted from sigstore-rs to use p256 instead of aws-lc-rs.
pub struct Keyring(HashMap<[u8; 32], Key>);

impl Keyring {
    /// Creates a `Keyring` from DER encoded SPKI-format public keys (no validity periods).
    pub fn new<'a>(keys: impl IntoIterator<Item = &'a [u8]>) -> Result<Self> {
        let parsed: Vec<Key> = keys.into_iter().map(Key::new).collect::<Result<Vec<_>>>()?;

        Ok(Self(
            parsed.into_iter().map(|k| (k.fingerprint, k)).collect(),
        ))
    }

    /// Creates a `Keyring` from DER encoded SPKI-format public keys with validity periods.
    ///
    /// Each item is (key_der, valid_from, valid_until) where timestamps are Unix seconds.
    pub fn new_with_validity<'a>(
        keys: impl IntoIterator<Item = (&'a [u8], Option<u64>, Option<u64>)>,
    ) -> Result<Self> {
        let parsed: Vec<Key> = keys
            .into_iter()
            .map(|(der, from, until)| Key::new(der).map(|k| k.with_validity(from, until)))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self(
            parsed.into_iter().map(|k| (k.fingerprint, k)).collect(),
        ))
    }

    /// Verifies `data` against a `signature` with a public key identified by `key_id`.
    pub fn verify(&self, key_id: &[u8; 32], signature: &[u8], data: &[u8]) -> Result<()> {
        let key = self.0.get(key_id).ok_or(KeyringError::KeyNotFound)?;

        let sig = Signature::from_der(signature).map_err(|_| KeyringError::VerificationFailed)?;

        key.inner
            .verify(data, &sig)
            .map_err(|_| KeyringError::VerificationFailed)?;

        Ok(())
    }

    /// Verifies `data` against a `signature`, checking that the key was valid at `timestamp`.
    ///
    /// `timestamp` is a Unix timestamp in seconds. The key's validity period (from the
    /// trust root) is checked before signature verification.
    pub fn verify_at(
        &self,
        key_id: &[u8; 32],
        signature: &[u8],
        data: &[u8],
        timestamp_secs: u64,
    ) -> Result<()> {
        let key = self.0.get(key_id).ok_or(KeyringError::KeyNotFound)?;

        // Check key validity period
        if let Some(from) = key.valid_from {
            if timestamp_secs < from {
                return Err(KeyringError::KeyNotValidAtTime);
            }
        }
        if let Some(until) = key.valid_until {
            if timestamp_secs > until {
                return Err(KeyringError::KeyNotValidAtTime);
            }
        }

        let sig = Signature::from_der(signature).map_err(|_| KeyringError::VerificationFailed)?;

        key.inner
            .verify(data, &sig)
            .map_err(|_| KeyringError::VerificationFailed)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{SigningKey, signature::Signer};
    use p256::pkcs8::EncodePublicKey;

    fn test_signing_key() -> SigningKey {
        // Well-known test scalar (RFC 6979 A.2.5)
        SigningKey::from_slice(&hex_literal::hex!(
            "C9AFA9D845BA75166B5C215767B1D6934E50C3DB36E89B127B8A622B120F6721"
        ))
        .expect("create test signing key")
    }

    fn test_keyring_and_id() -> (Keyring, [u8; 32]) {
        let sk = test_signing_key();
        let vk = sk.verifying_key();
        let spki_der = vk.to_public_key_der().expect("encode SPKI");
        let keyring = Keyring::new([spki_der.as_bytes()]).expect("create keyring");

        // Compute key fingerprint (SHA-256 of SPKI DER)
        let fingerprint: [u8; 32] = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(spki_der.as_bytes());
            h.finalize().into()
        };

        (keyring, fingerprint)
    }

    #[test]
    fn test_keyring_key_not_found() {
        let keyring = Keyring(HashMap::new());
        let result = keyring.verify(&[0u8; 32], &[], &[]);
        assert!(matches!(result, Err(KeyringError::KeyNotFound)));
    }

    #[test]
    fn test_keyring_rejects_malformed_key() {
        let bad_key: &[u8] = &[0x00, 0x01, 0x02, 0x03];
        let result = Keyring::new([bad_key]);
        assert!(result.is_err());
    }

    #[test]
    fn test_keyring_with_validity_rejects_malformed_key() {
        let bad_key: &[u8] = &[0x00, 0x01, 0x02];
        let result = Keyring::new_with_validity([(bad_key, None, None)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_success() {
        let sk = test_signing_key();
        let (keyring, key_id) = test_keyring_and_id();
        let message = b"some message";
        let sig: p256::ecdsa::DerSignature = sk.sign(message);
        assert!(keyring.verify(&key_id, sig.as_bytes(), message).is_ok());
    }

    #[test]
    fn test_verify_wrong_key_id() {
        let sk = test_signing_key();
        let (keyring, _) = test_keyring_and_id();
        let message = b"some message";
        let sig: p256::ecdsa::DerSignature = sk.sign(message);
        let result = keyring.verify(&[0u8; 32], sig.as_bytes(), message);
        assert!(matches!(result, Err(KeyringError::KeyNotFound)));
    }

    #[test]
    fn test_verify_wrong_payload() {
        let sk = test_signing_key();
        let (keyring, key_id) = test_keyring_and_id();
        let sig: p256::ecdsa::DerSignature = sk.sign(b"some message");
        let result = keyring.verify(&key_id, sig.as_bytes(), b"different message");
        assert!(matches!(result, Err(KeyringError::VerificationFailed)));
    }

    #[test]
    fn test_verify_wrong_keyring() {
        let sk = test_signing_key();
        let message = b"some message";
        let sig: p256::ecdsa::DerSignature = sk.sign(message);

        // Build a keyring with a different key
        let other_sk = SigningKey::from_slice(&hex_literal::hex!(
            "0000000000000000000000000000000000000000000000000000000000000001"
        ))
        .expect("create other key");
        let other_vk = other_sk.verifying_key();
        let other_spki = other_vk.to_public_key_der().expect("encode SPKI");
        let other_keyring = Keyring::new([other_spki.as_bytes()]).expect("create keyring");

        // The key_id from the original key won't be found in other_keyring
        let (_, key_id) = test_keyring_and_id();
        let result = other_keyring.verify(&key_id, sig.as_bytes(), message);
        assert!(matches!(result, Err(KeyringError::KeyNotFound)));
    }

    #[test]
    fn test_verify_at_before_validity() {
        let sk = test_signing_key();
        let vk = sk.verifying_key();
        let spki_der = vk.to_public_key_der().expect("encode SPKI");

        let keyring = Keyring::new_with_validity([(
            spki_der.as_bytes(),
            Some(1000), // valid_from
            Some(2000), // valid_until
        )])
        .expect("create keyring");

        let fingerprint: [u8; 32] = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(spki_der.as_bytes());
            h.finalize().into()
        };

        let message = b"some message";
        let sig: p256::ecdsa::DerSignature = sk.sign(message);

        let result = keyring.verify_at(&fingerprint, sig.as_bytes(), message, 999);
        assert!(matches!(result, Err(KeyringError::KeyNotValidAtTime)));
    }

    #[test]
    fn test_verify_at_after_validity() {
        let sk = test_signing_key();
        let vk = sk.verifying_key();
        let spki_der = vk.to_public_key_der().expect("encode SPKI");

        let keyring = Keyring::new_with_validity([(spki_der.as_bytes(), Some(1000), Some(2000))])
            .expect("create keyring");

        let fingerprint: [u8; 32] = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(spki_der.as_bytes());
            h.finalize().into()
        };

        let message = b"some message";
        let sig: p256::ecdsa::DerSignature = sk.sign(message);

        let result = keyring.verify_at(&fingerprint, sig.as_bytes(), message, 2001);
        assert!(matches!(result, Err(KeyringError::KeyNotValidAtTime)));
    }
}

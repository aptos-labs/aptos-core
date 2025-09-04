// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides an API for the PureEdDSA signature scheme over the Ed25519 twisted
//! Edwards curve as defined in [RFC8032](https://tools.ietf.org/html/rfc8032).
//!
//! Signature verification also checks and rejects non-canonical signatures.
//!
//! # Examples
//!
//! ```
//! use velor_crypto_derive::{CryptoHasher, BCSCryptoHash};
//! use velor_crypto::{
//!     ed25519::*,
//!     traits::{Signature, SigningKey, Uniform},
//!     test_utils::KeyPair
//! };
//! use rand::{rngs::StdRng, SeedableRng};
//! use rand_core::OsRng;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
//! pub struct TestCryptoDocTest(String);
//! let message = TestCryptoDocTest("Test message".to_string());
//!
//! let mut rng = OsRng;
//! let kp = KeyPair::<Ed25519PrivateKey, Ed25519PublicKey>::generate(&mut rng);
//!
//! let signature = kp.private_key.sign(&message).unwrap();
//! assert!(signature.verify(&message, &kp.public_key).is_ok());
//! ```

/// The length of the Ed25519PrivateKey
pub const ED25519_PRIVATE_KEY_LENGTH: usize = ed25519_dalek::SECRET_KEY_LENGTH;
/// The length of the Ed25519PublicKey
pub const ED25519_PUBLIC_KEY_LENGTH: usize = ed25519_dalek::PUBLIC_KEY_LENGTH;
/// The length of the Ed25519Signature
pub const ED25519_SIGNATURE_LENGTH: usize = ed25519_dalek::SIGNATURE_LENGTH;

/// The order of ed25519 as defined in [RFC8032](https://tools.ietf.org/html/rfc8032).
const L: [u8; 32] = [
    0xED, 0xD3, 0xF5, 0x5C, 0x1A, 0x63, 0x12, 0x58, 0xD6, 0x9C, 0xF7, 0xA2, 0xDE, 0xF9, 0xDE, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

pub mod ed25519_keys;
pub mod ed25519_sigs;

#[cfg(any(test, feature = "fuzzing"))]
pub use ed25519_keys::keypair_strategy;
pub use ed25519_keys::{
    Ed25519PrivateKey, Ed25519PrivateKey as PrivateKey, Ed25519PublicKey,
    Ed25519PublicKey as PublicKey,
};
pub use ed25519_sigs::{Ed25519Signature, Ed25519Signature as Signature};

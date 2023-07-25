// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
// TODO: Fix docs
//! This module provides an API for the PureEdDSA signature scheme over the Ed25519 twisted
//! Edwards curve as defined in [RFC8032](https://tools.ietf.org/html/rfc8032).
//!
//! Signature verification also checks and rejects non-canonical signatures.
//!
//! # Examples
//!
//! ```
//! use aptos_crypto_derive::{CryptoHasher, BCSCryptoHash};
//! use aptos_crypto::{
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

/// The length of the P256PrivateKey
pub const P256_PRIVATE_KEY_LENGTH: usize = 32;
/// The length of the P256PublicKey
pub const P256_PUBLIC_KEY_LENGTH: usize = 64;
/// The length of the P256Signature
pub const P256_SIGNATURE_LENGTH: usize = 64;

/// The order of p256 as defined in [NIST SP 800-186](https://csrc.nist.gov/publications/detail/sp/800-186/final).
// TODO: Make sure this has the correct endianness
/*const L: [u8; 32] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xBC, 0xE6, 0xFA, 0xAD, 0xA7, 0x17, 0x9e, 0x84, 0xF3, 0xB9, 0xCA, 0xC2, 0xFC, 0x63, 0x25, 0x51,
];*/

pub mod p256_keys;
pub mod p256_sigs;

#[cfg(any(test, feature = "fuzzing"))]
pub use p256_keys::keypair_strategy;
pub use p256_keys::{
    P256PrivateKey, P256PrivateKey as PrivateKey, P256PublicKey,
    P256PublicKey as PublicKey,
};
pub use p256_sigs::{P256Signature, P256Signature as Signature};

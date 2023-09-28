// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//! This module provides an API for the ECDSA signature scheme over the NIST-P256 curve as defined in [NIST SP 800-186](https://csrc.nist.gov/publications/detail/sp/800-186/final).
//!
//! Signature verification also checks and rejects non-canonical signatures. Signing is guaranteed
//! to output the canonical signature which passes this module's verification. 
//!
//! # Examples
//!
//! ```
//! use aptos_crypto_derive::{CryptoHasher, BCSCryptoHash};
//! use aptos_crypto::{
//!     p256::*,
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
//! let kp = KeyPair::<P256PrivateKey, P256PublicKey>::generate(&mut rng);
//!
//! let signature = kp.private_key.sign(&message).unwrap();
//! assert!(signature.verify(&message, &kp.public_key).is_ok());
//! ```

/// The length in bytes of the P256PrivateKey
pub const P256_PRIVATE_KEY_LENGTH: usize = 32;
/// The length in bytes of the P256PublicKey
pub const P256_PUBLIC_KEY_LENGTH: usize = 65;
/// The length in bytes of the P256Signature
pub const P256_SIGNATURE_LENGTH: usize = 64;

/// The order of P256 as defined in [NIST SP 800-186](https://csrc.nist.gov/publications/detail/sp/800-186/final), minus one.
const ORDER_MINUS_ONE: [u8; 32] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xBC, 0xE6, 0xFA, 0xAD, 0xA7, 0x17, 0x9E, 0x84, 0xF3, 0xB9, 0xCA, 0xC2, 0xFC, 0x63, 0x25, 0x50,
];

/// The value (q-1)/2, where q is the order of P256 as defined in [NIST SP 800-186](https://csrc.nist.gov/publications/detail/sp/800-186/final).
/// Computed with the following SageMath code:
///
/// # Curve order
/// qq = 0xFFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551
/// q_half = (qq-1)/2
pub const ORDER_HALF: [u8; 32] = [
    0x7F, 0xFF, 0xFF, 0xFF, 0x80, 0x00, 0x00, 0x00, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xDE, 0x73, 0x7D, 0x56, 0xD3, 0x8B, 0xCF, 0x42, 0x79, 0xDC, 0xE5, 0x61, 0x7E, 0x31, 0x92, 0xA8,
];

pub mod p256_keys;
pub mod p256_sigs;

#[cfg(any(test, feature = "fuzzing"))]
pub use p256_keys::keypair_strategy;
pub use p256_keys::{
    P256PrivateKey, P256PrivateKey as PrivateKey, P256PublicKey,
    P256PublicKey as PublicKey,
};
pub use p256_sigs::{P256Signature, P256Signature as Signature};

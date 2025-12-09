// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//! This module provides an API for the SLH-DSA (SPHINCS+) signature scheme using the SHA2-128s variant
//! as described in [FIPS-205](https://csrc.nist.gov/publications/detail/fips/205/final).
//!
//! SLH-DSA is a stateless hash-based signature scheme that provides post-quantum security.
//!
//! # Examples
//!
//! ```
//! use aptos_crypto_derive::{CryptoHasher, BCSCryptoHash};
//! use aptos_crypto::{
//!     slh_dsa_sha2_128s::*,
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
//! let kp = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);
//!
//! let signature = kp.private_key.sign(&message).unwrap();
//! assert!(signature.verify(&message, &kp.public_key).is_ok());
//! ```

// Constants are defined directly below, no imports needed here

// SLH-DSA SHA2-128s key and signature lengths
// These are based on the FIPS-205 specification for SHA2-128s
/// The length in bytes of the SLH-DSA SHA2-128s PrivateKey (seed)
pub const PRIVATE_KEY_LENGTH: usize = 32;
/// The length in bytes of the SLH-DSA SHA2-128s PublicKey
// For SHA2-128s, the public key is 32 bytes
pub const PUBLIC_KEY_LENGTH: usize = 32;
/// The length in bytes of the SLH-DSA SHA2-128s Signature
// For SHA2-128s, the signature is 7856 bytes (succinct variant)
pub const SIGNATURE_LENGTH: usize = 7856;

pub mod slh_dsa_keys;
pub mod slh_dsa_sigs;

#[cfg(any(test, feature = "fuzzing"))]
pub use slh_dsa_keys::keypair_strategy;
pub use slh_dsa_keys::{PrivateKey, PublicKey};
pub use slh_dsa_sigs::Signature;

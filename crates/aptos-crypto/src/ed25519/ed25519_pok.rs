//! This file implements a proof-of-knowledge for the private key associated with an Ed25519 public key.
//!
//! # Examples
//!
//! ```
//! use aptos_crypto::{
//!     ed25519::*,
//!     traits::{Signature, SigningKey, Uniform},
//!     test_utils::KeyPair
//! };
//! use rand::{rngs::StdRng, SeedableRng};
//! use rand_core::OsRng;
//!
//! let mut rng = OsRng;
//! let kp = KeyPair::<Ed25519PrivateKey, Ed25519PublicKey>::generate(&mut rng);
//!
//! let pok = kp.private_key.create_proof_of_knowledge();
//! assert!(kp.public_key.verify_proof_of_knowledge(&pok).is_ok());
//! ```

use crate::ed25519::Ed25519PublicKey;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use curve25519_dalek::edwards::CompressedEdwardsY;
use serde::{Deserialize, Serialize};

/// The challenge message for a proof-of-knowledge (PoK) of an Ed25519 private key
#[derive(Debug, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub(crate) struct Ed25519PoKChallenge(pub(crate) Ed25519PublicKey, pub(crate) CompressedEdwardsY);

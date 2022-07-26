// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! A library supplying various cryptographic primitives
pub mod bls12381;
pub mod compat;
pub mod ed25519;
pub mod error;
pub mod hash;
pub mod hkdf;
pub mod multi_ed25519;
pub mod noise;
pub mod test_utils;
pub mod traits;
pub mod validatable;
pub mod x25519;

#[cfg(test)]
mod unit_tests;

pub use self::traits::*;
pub use hash::HashValue;

// We need to add this here if we want to use aptos-crypto-derive's CryptoHasher and BCSCryptoHasher
// macros because these macros generate a `use aptos_crypto::hash::CryptoHash` line which will fail
// inside this crate (i.e., it would need to be a `use crate::hash::CryptoHash` line instead).
extern crate self as aptos_crypto;

// Reexport once_cell and serde_name for use in CryptoHasher Derive implementation.
#[doc(hidden)]
pub use once_cell as _once_cell;
#[doc(hidden)]
pub use serde_name as _serde_name;

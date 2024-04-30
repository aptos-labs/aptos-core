#![cfg_attr(not(feature = "std"), no_std)]

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub extern crate derivation_path;
pub extern crate ed25519_dalek;

pub use derivation_path::{ChildIndex, DerivationPath};
pub use ed25519_dalek::{PublicKey, SecretKey};

use core::fmt;
use hmac::{Hmac, Mac};
use sha2_0_10_6::Sha512;
use regex::Regex;
use std::str::FromStr;
use anyhow::{bail, Result};


const PEPPER_SLIP_10_NAME: &str = "32 bytes";

/// Errors thrown while deriving secret keys
#[derive(Debug)]
pub enum Error {
    Ed25519,
    ExpectedHardenedIndex(ChildIndex),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ed25519 => f.write_str("ed25519 error"),
            Self::ExpectedHardenedIndex(index) => {
                f.write_fmt(format_args!("expected hardened child index: {}", index))
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

/// An expanded secret key with chain code and meta data
#[derive(Debug)]
pub struct ExtendedPepper {
    /// How many derivations this key is from the root (0 for root)
    pub depth: u8,
    /// Child index of the key used to derive from parent (`Normal(0)` for root)
    pub child_index: ChildIndex,
    /// Pepper
    pub pepper: [u8; 32],
    /// Chain code
    pub chain_code: [u8; 32],
}

type HmacSha512 = Hmac<Sha512>;

/// A convenience wrapper for a [`core::result::Result`] with an [`Error`]
// pub type Result<T, E = Error> = core::result::Result<T, E>;

pub fn get_aptos_derivation_path(s: &str) -> Result<DerivationPath> {
    let re = Regex::new(r"^m\/44'\/637'\/[0-9]+'\/[0-9]+'\/[0-9]+'?$").unwrap();
    if re.is_match(s) {
        println!("Valid path");
    } else {
        bail!(format!("Invalid derivation path: {}", s))
    }
    Ok(DerivationPath::from_str(s)?)
}

impl ExtendedPepper {
    pub fn get_pepper(&self) -> [u8; 31] {
        let mut pepper = [0; 31];
        pepper.copy_from_slice(&self.pepper[..31]);
        pepper
    }

    /// Create a new extended secret key from a seed
    pub fn from_seed(seed: &[u8]) -> Result<Self> {
        let mut mac = HmacSha512::new_from_slice(PEPPER_SLIP_10_NAME.as_ref()).unwrap();
        mac.update(seed);
        let bytes = mac.finalize().into_bytes();

        let mut pepper = [0; 32];
        pepper.copy_from_slice(&bytes[..32]);
        let mut chain_code = [0; 32];
        chain_code.copy_from_slice(&bytes[32..]);

        Ok(Self {
            depth: 0,
            child_index: ChildIndex::Normal(0),
            pepper,
            chain_code,
        })
    }

    /// Derive an extended secret key fom the current using a derivation path
    pub fn derive<P: AsRef<[ChildIndex]>>(&self, path: &P) -> Result<Self> {
        let mut path = path.as_ref().iter();
        let mut next = match path.next() {
            Some(index) => self.derive_child(*index)?,
            None => self.clone(),
        };
        for index in path {
            next = next.derive_child(*index)?;
        }
        Ok(next)
    }

    /// Derive a child extended secret key with an index
    pub fn derive_child(&self, index: ChildIndex) -> Result<Self> {
        if index.is_normal() {
            bail!(format!("expected hardened child index: {}", index))
        }

        let mut mac = HmacSha512::new_from_slice(&self.chain_code).unwrap();
        mac.update(&[0u8]);
        mac.update(self.pepper.as_ref());
        mac.update(index.to_bits().to_be_bytes().as_ref());
        let bytes = mac.finalize().into_bytes();

        let mut pepper = [0; 32];
        pepper.copy_from_slice(&bytes[..32]);
        let mut chain_code = [0; 32];
        chain_code.copy_from_slice(&bytes[32..]);

        Ok(Self {
            depth: self.depth + 1,
            child_index: index,
            pepper,
            chain_code,
        })
    }

    #[inline]
    fn clone(&self) -> Self {
        Self {
            depth: self.depth,
            child_index: self.child_index,
            pepper: self.pepper,
            chain_code: self.chain_code,
        }
    }
}

impl From<ed25519_dalek::SignatureError> for Error {
    fn from(_: ed25519_dalek::SignatureError) -> Self {
        Self::Ed25519
    }
}

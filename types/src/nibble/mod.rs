// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! `Nibble` represents a four-bit unsigned integer.

pub mod nibble_path;

use aptos_crypto::HashValue;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The hardcoded maximum height of a state merkle tree in nibbles.
pub const ROOT_NIBBLE_HEIGHT: usize = HashValue::LENGTH * 2;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Nibble(u8);

impl Nibble {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl From<u8> for Nibble {
    fn from(nibble: u8) -> Self {
        assert!(nibble < 16, "Nibble out of range: {}", nibble);
        Self(nibble)
    }
}

impl From<Nibble> for u8 {
    fn from(nibble: Nibble) -> Self {
        nibble.0
    }
}

impl From<Nibble> for usize {
    fn from(nibble: Nibble) -> Self {
        nibble.0 as usize
    }
}

impl fmt::LowerHex for Nibble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for Nibble {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (0..16u8).prop_map(Self::from).boxed()
    }
}

pub trait ExpectNibble {
    fn expect_nibble(&self) -> Nibble;
}

impl ExpectNibble for usize {
    fn expect_nibble(&self) -> Nibble {
        Nibble(*self as u8)
    }
}

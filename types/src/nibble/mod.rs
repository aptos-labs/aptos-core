// Copyright (c) Aptos
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
pub const ROOT_NIBBLE_HEIGHT: usize = HashValue::LENGTH * 8 / NIBBLE_SIZE_IN_BITS;
pub const NIBBLE_SIZE_IN_BITS: usize = 4;
pub const MAX_NIBBLE: usize = 1 << NIBBLE_SIZE_IN_BITS;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Nibble(usize);

impl Nibble {
    pub(crate) fn get_bit(&self, i: usize) -> bool {
        assert!(i < NIBBLE_SIZE_IN_BITS);
        let mask = 1 << (7 - i);
        self.0 & mask != 0
    }
}

impl From<Nibble> for usize {
    fn from(n: Nibble) -> Self {
        n.0
    }
}

impl From<usize> for Nibble {
    fn from(v: usize) -> Self {
        Nibble(v)
    }
}

impl From<u8> for Nibble {
    fn from(nibble: u8) -> Self {
        assert!(
            (nibble as usize) < MAX_NIBBLE,
            "Nibble out of range: {}",
            nibble
        );
        Self(nibble as usize)
    }
}

impl From<&[bool]> for Nibble {
    fn from(bits: &[bool]) -> Self {
        assert_eq!(bits.len(), NIBBLE_SIZE_IN_BITS);
        let mut val = 0;
        for &bit in bits {
            val = (val << 1) + bit as usize
        }
        Nibble(val)
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
        (0..MAX_NIBBLE).prop_map(Self::from).boxed()
    }
}

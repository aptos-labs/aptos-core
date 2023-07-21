// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct Randomness {
    // dkg todo: fill in the fields
    pub dummy_bytes: Vec<u8>,
}

impl Randomness {
    pub fn new() -> Self {
        Self {
            dummy_bytes: vec![0],
        }
    }
}

impl Default for Randomness {
    fn default() -> Self {
        Self::new()
    }
}

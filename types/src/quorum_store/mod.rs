// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
};

/// A unique identifier for a batch of transactions in quorum store
#[derive(
    Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, CryptoHasher, BCSCryptoHash,
)]
pub struct BatchId {
    pub id: u64,
    /// A number that is stored in the DB and updated only if the value does not exist in
    /// the DB: (a) at the start of an epoch, or (b) the DB was wiped. When the nonce is updated,
    /// id starts again at 0. Using the current system time allows the nonce to be ordering.
    pub nonce: u64,
}

impl BatchId {
    pub fn new(nonce: u64) -> Self {
        Self { id: 0, nonce }
    }

    pub fn new_for_test(id: u64) -> Self {
        Self { id, nonce: 0 }
    }

    pub fn increment(&mut self) {
        self.id += 1;
    }
}

impl PartialOrd<Self> for BatchId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BatchId {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.nonce.cmp(&other.nonce) {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        self.id.cmp(&other.id)
    }
}

impl Display for BatchId {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "({}, {})", self.id, self.nonce)
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum SystemTransaction {
    DummyTopic(DummySystemTransaction),
    // to be populated...
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct DummySystemTransaction {
    pub nonce: u64,
}

impl SystemTransaction {
    pub fn size_in_bytes(&self) -> usize {
        match self {
            SystemTransaction::DummyTopic(_) => 16 // Better over-claim?
        }
    }
}

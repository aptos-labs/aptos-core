// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum SystemTransaction {
    #[cfg(any(test, feature = "fuzzing"))]
    DummyTopic(DummySystemTransaction),
    // to be populated...
}

#[cfg(any(test, feature = "fuzzing"))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct DummySystemTransaction {
    pub payload: Vec<u8>,
}

impl SystemTransaction {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy(payload: Vec<u8>) -> Self {
        Self::DummyTopic(DummySystemTransaction { payload })
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn size_in_bytes(&self) -> usize {
        match self {
            SystemTransaction::DummyTopic(txn) => txn.payload.len(),
        }
    }

    #[cfg(not(any(test, feature = "fuzzing")))]
    pub fn size_in_bytes(&self) -> usize {
        0
    }
}

pub mod pool;

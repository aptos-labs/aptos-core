// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum ValidatorTransaction {
    #[cfg(any(test, feature = "fuzzing"))]
    DummyTopic(DummyValidatorTransaction),
    // to be populated...
}

#[cfg(any(test, feature = "fuzzing"))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct DummyValidatorTransaction {
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl ValidatorTransaction {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy(payload: Vec<u8>) -> Self {
        Self::DummyTopic(DummyValidatorTransaction { payload })
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn size_in_bytes(&self) -> usize {
        match self {
            ValidatorTransaction::DummyTopic(txn) => txn.payload.len(),
        }
    }

    #[cfg(not(any(test, feature = "fuzzing")))]
    pub fn size_in_bytes(&self) -> usize {
        0
    }
}

pub mod pool;

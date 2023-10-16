// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::CurrentTimeMicroseconds;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};

#[derive(
    BCSCryptoHash,
    Clone,
    CryptoHasher,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    Ord,
    PartialOrd,
    Hash,
)]
pub enum StateValueMetadata {
    V0 {
        deposit: u64,
        creation_time_usecs: u64,
    },
}

impl StateValueMetadata {
    pub fn new(deposit: u64, creation_time_usecs: &CurrentTimeMicroseconds) -> Self {
        Self::V0 {
            deposit,
            creation_time_usecs: creation_time_usecs.microseconds,
        }
    }

    pub fn deposit(&self) -> u64 {
        match self {
            StateValueMetadata::V0 { deposit, .. } => *deposit,
        }
    }

    pub fn set_deposit(&mut self, amount: u64) {
        match self {
            StateValueMetadata::V0 { deposit, .. } => *deposit = amount,
        }
    }
}

// To avoid nested options when fetching a resource and its metadata.
pub type StateValueMetadataKind = Option<StateValueMetadata>;

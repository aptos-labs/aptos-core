// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_types::{state_store::state_storage_usage::StateStorageUsage, transaction::Version};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum MetadataValue {
    Version(Version),
    StateSnapshotProgress(StateSnapshotProgress),
}

impl MetadataValue {
    pub fn expect_version(self) -> Version {
        match self {
            Self::Version(v) => v,
            _ => panic!("Not version"),
        }
    }

    pub fn expect_state_snapshot_progress(self) -> StateSnapshotProgress {
        match self {
            Self::StateSnapshotProgress(p) => p,
            _ => panic!("Not state snapshot progress"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Hash, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum MetadataKey {
    LatestVersion,
    EventPrunerProgress,
    TransactionPrunerProgress,
    StateSnapshotRestoreProgress(Version),
    EventVersion,
    StateVersion,
    TransactionVersion,
    EventV2TranslationVersion,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct StateSnapshotProgress {
    pub key_hash: HashValue,
    pub usage: StateStorageUsage,
}

impl StateSnapshotProgress {
    pub fn new(key_hash: HashValue, usage: StateStorageUsage) -> Self {
        Self { key_hash, usage }
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::Version;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum MetadataValue {
    Version(Version),
    Timestamp(u64),
}

impl MetadataValue {
    pub fn expect_version(self) -> Version {
        match self {
            Self::Version(v) => v,
            _ => panic!("Expected MetadataValue::Version"),
        }
    }

    pub fn last_restored_timestamp(self) -> u64 {
        match self {
            Self::Timestamp(t) => t,
            _ => panic!("Expected MetadataValue::Timestamp"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum MetadataKey {
    LatestVersion,
    RestoreTimestamp,
}

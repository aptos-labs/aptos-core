// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// This file is a copy of the file storage/indexer/src/metadata.rs.
/// At the end of the migration to migrate table info mapping
/// from storage critical path to indexer, the other file will be removed.
use aptos_types::transaction::Version;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub(crate) enum MetadataValue {
    Version(Version),
}

impl MetadataValue {
    pub fn expect_version(self) -> Version {
        match self {
            Self::Version(v) => v,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub(crate) enum MetadataKey {
    LatestVersion,
}

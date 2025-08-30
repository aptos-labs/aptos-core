// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{ExecutionStatus, Version};
use serde::{Deserialize, Serialize};

/// A filter that determines what transaction output differences are acceptable.
#[derive(Deserialize, Serialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DiffFilter {
    /// Allows gas usage differences within specified bounds.
    GasChange {
        /// If set to some value X, allows gas delta (gas used after - gas used before) to be at
        /// most X.
        #[serde(skip_serializing_if = "Option::is_none")]
        min_delta: Option<i64>,
        /// If set to some value X, allows gas delta to be at least X only.
        #[serde(skip_serializing_if = "Option::is_none")]
        max_delta: Option<i64>,
    },
    /// Allows status change but verifies the rest of the output (writes, events, gas).
    SoftStatusChange {
        from: ExecutionStatus,
        to: ExecutionStatus,
    },
    /// Allows status change and skips any subsequent verification, treating the diff as empty.
    HardStatusChange {
        from: ExecutionStatus,
        to: ExecutionStatus,
    },
}

/// Version range for applying filters to specific transaction ranges (start and end inclusive).
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct VersionRange {
    pub start: Version,
    pub end: Version,
}

/// Filter used for a versioned chunk of transactions. If no version range is provided, applied to
/// all transactions.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChunkFilter {
    pub filter: DiffFilter,
    pub range: Option<VersionRange>,
}

impl ChunkFilter {
    /// Check if this filter applies to the given version
    pub fn applies_to_version(&self, version: Version) -> bool {
        match &self.range {
            Some(range) => version >= range.start && version <= range.end,
            // If there is no range, the filter applies to all versions.
            None => true,
        }
    }
}

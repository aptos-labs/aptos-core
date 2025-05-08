// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Struct that represents a NewEpochEvent.
#[derive(Debug, Serialize, Deserialize)]
pub struct NewEpochEvent {
    epoch: u64,
}

impl NewEpochEvent {
    // #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy() -> Self {
        Self { epoch: 0 }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}

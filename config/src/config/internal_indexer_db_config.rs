// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct InternalIndexerDBConfig {
    pub enable_transaction: bool,
    pub enable_event: bool,
    pub enable_statekeys: bool,
    pub batch_size: usize,
}

impl InternalIndexerDBConfig {
    pub fn new(
        enable_transaction: bool,
        enable_event: bool,
        enable_statekeys: bool,
        batch_size: usize,
    ) -> Self {
        Self {
            enable_transaction,
            enable_event,
            enable_statekeys,
            batch_size,
        }
    }

    pub fn enable_transaction(&self) -> bool {
        self.enable_transaction
    }

    pub fn enable_event(&self) -> bool {
        self.enable_event
    }

    pub fn enable_statekeys(&self) -> bool {
        self.enable_statekeys
    }

    pub fn is_internal_indexer_db_enabled(&self) -> bool {
        self.enable_transaction || self.enable_event || self.enable_statekeys
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }
}

impl Default for InternalIndexerDBConfig {
    fn default() -> Self {
        Self {
            enable_transaction: false,
            enable_event: false,
            enable_statekeys: false,
            batch_size: 10_000,
        }
    }
}

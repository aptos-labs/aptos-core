// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct InternalIndexerDBConfig {
    pub enable_transaction: bool,
    pub enable_event: bool,
    pub batch_size: usize,
}

impl InternalIndexerDBConfig {
    pub fn new(enable_transaction: bool, enable_event: bool, batch_size: usize) -> Self {
        Self {
            enable_transaction,
            enable_event,
            batch_size,
        }
    }

    pub fn enable_transaction(&self) -> bool {
        self.enable_transaction
    }

    pub fn enable_event(&self) -> bool {
        self.enable_event
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
            batch_size: 10_000,
        }
    }
}

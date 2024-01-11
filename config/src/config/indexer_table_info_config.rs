// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

// Useful defaults
pub const DEFAULT_PARSER_TASK_COUNT: u16 = 20;
pub const DEFAULT_PARSER_BATCH_SIZE: u16 = 1000;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexerTableInfoConfig {
    /// Enable table info parsing
    pub enabled: bool,

    /// Number of processor tasks to fan out
    pub parser_task_count: u16,

    /// Number of transactions each parser will process
    pub parser_batch_size: u16,

    pub enable_expensive_logging: bool,
}

// Reminder, #[serde(default)] on IndexerTableInfoConfig means that the default values for
// fields will come from this Default impl, unless the field has a specific
// #[serde(default)] on it (which none of the above do).
impl Default for IndexerTableInfoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            parser_task_count: DEFAULT_PARSER_TASK_COUNT,
            parser_batch_size: DEFAULT_PARSER_BATCH_SIZE,
            enable_expensive_logging: false,
        }
    }
}

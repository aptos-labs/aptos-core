// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

// Useful defaults
pub const DEFAULT_PARSER_TASK_COUNT: u16 = 20;
pub const DEFAULT_PARSER_BATCH_SIZE: u16 = 1000;
pub const DEFAULT_TABLE_INFO_BUCKET: &str = "default-table-info";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum TableInfoServiceMode {
    /// Backup service mode with GCS bucket name.
    Backup(String),
    /// Restore service mode with GCS bucket name.
    Restore(String),
    IndexingOnly,
    Disabled,
}

impl TableInfoServiceMode {
    pub fn is_enabled(&self) -> bool {
        !matches!(self, TableInfoServiceMode::Disabled)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexerTableInfoConfig {
    /// Number of processor tasks to fan out
    pub parser_task_count: u16,

    /// Number of transactions each parser will process
    pub parser_batch_size: u16,
    pub table_info_service_mode: TableInfoServiceMode,
}

// Reminder, #[serde(default)] on IndexerTableInfoConfig means that the default values for
// fields will come from this Default impl, unless the field has a specific
// #[serde(default)] on it (which none of the above do).
impl Default for IndexerTableInfoConfig {
    fn default() -> Self {
        Self {
            parser_task_count: DEFAULT_PARSER_TASK_COUNT,
            parser_batch_size: DEFAULT_PARSER_BATCH_SIZE,
            table_info_service_mode: TableInfoServiceMode::Disabled,
        }
    }
}

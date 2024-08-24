// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

// Useful defaults
pub const DEFAULT_PARSER_TASK_COUNT: u16 = 20;
pub const DEFAULT_PARSER_BATCH_SIZE: u16 = 1000;
pub const DEFAULT_TABLE_INFO_BUCKET: &str = "default-table-info";
pub const DEFAULT_BUCKET_NAME: &str = "table-info";
pub const DEFAULT_VERSION_DIFF: u64 = 100_000;

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

    /// Enable backup service
    pub db_backup_enabled: bool,

    /// Backup and restore service config
    pub gcs_bucket_name: String,

    /// if version difference btw this FN and latest ledger version is less than VERSION_DIFF
    /// do not restore, start from the FN version, to avoid time consuming db restore from gcs
    pub version_diff: u64,
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
            db_backup_enabled: false,
            gcs_bucket_name: DEFAULT_TABLE_INFO_BUCKET.to_owned(),
            version_diff: DEFAULT_VERSION_DIFF,
        }
    }
}

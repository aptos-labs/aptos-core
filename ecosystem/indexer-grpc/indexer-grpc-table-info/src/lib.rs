// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod backup_restore;
pub mod internal_indexer_db_service;
pub mod metrics;
pub mod runtime;
pub mod table_info_service;

/// Snapshot folder prefix for a chain; this is used to identify the snapshot folder and backup.
pub fn snapshot_folder_prefix(chain_id: u64) -> String {
    format!("snapshot_chain_{}_epoch_", chain_id)
}

/// Snapshot folder name for a chain and epoch.
pub fn snapshot_folder_name(chain_id: u64, epoch: u64) -> String {
    format!("snapshot_chain_{}_epoch_{}", chain_id, epoch)
}

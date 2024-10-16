// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod backup_restore;
pub mod internal_indexer_db_service;
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

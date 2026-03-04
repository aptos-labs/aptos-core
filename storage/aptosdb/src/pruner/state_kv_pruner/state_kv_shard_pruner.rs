// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    pruner::pruner_utils::get_or_initialize_subpruner_progress,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        stale_state_value_index_by_key_hash::StaleStateValueIndexByKeyHashSchema,
        state_value_by_key_hash::StateValueByKeyHashSchema,
    },
};
use aptos_logger::info;
use aptos_schemadb::{batch::SchemaBatch, ReadOptions, DB};
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::sync::Arc;

// Per-shard pruner for state KV data
pub(in crate::pruner) struct StateKvShardPruner {
    shard_id: usize,
    db_shard: Arc<DB>,
}

impl StateKvShardPruner {
    pub(in crate::pruner) fn new(
        shard_id: usize,
        db_shard: Arc<DB>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            &db_shard,
            &DbMetadataKey::StateKvShardPrunerProgress(shard_id),
            metadata_progress,
        )?;
        let myself = Self { shard_id, db_shard };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up state kv shard {shard_id}."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }

    pub(in crate::pruner) fn prune(
        &self,
        current_progress: Version,
        target_version: Version,
    ) -> Result<()> {
        let mut batch = SchemaBatch::new();

        let mut read_opts = ReadOptions::default();
        read_opts.fill_cache(false);
        let mut iter = self
            .db_shard
            .iter_with_opts::<StaleStateValueIndexByKeyHashSchema>(read_opts)?;
        iter.seek(&current_progress)?;
        for item in iter {
            let (index, _) = item?;
            if index.stale_since_version > target_version {
                break;
            }
            batch.delete::<StaleStateValueIndexByKeyHashSchema>(&index)?;
            batch.delete::<StateValueByKeyHashSchema>(&(index.state_key_hash, index.version))?;
        }
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvShardPrunerProgress(self.shard_id),
            &DbMetadataValue::Version(target_version),
        )?;

        self.db_shard.write_schemas(batch)
    }

    pub(in crate::pruner) fn shard_id(&self) -> usize {
        self.shard_id
    }
}

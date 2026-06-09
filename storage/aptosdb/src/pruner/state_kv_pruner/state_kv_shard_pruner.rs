// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    pruner::{
        pruner_utils::get_or_initialize_subpruner_progress,
        state_kv_pruner::generics::StateValuePrunerSchema,
    },
    schema::db_metadata::{DbMetadataSchema, DbMetadataValue},
};
use aptos_logger::info;
use aptos_schemadb::{batch::SchemaBatch, schema::SeekKeyCodec, ReadOptions, DB};
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::{marker::PhantomData, sync::Arc};

// Per-shard pruner for state value data (main-state cold/hot or position).
pub(in crate::pruner) struct StateKvShardPruner<S> {
    shard_id: usize,
    db_shard: Arc<DB>,
    _phantom: PhantomData<S>,
}

impl<S: StateValuePrunerSchema> StateKvShardPruner<S>
where
    Version: SeekKeyCodec<S::StaleIndexSchema>,
{
    pub(in crate::pruner) fn new(
        shard_id: usize,
        db_shard: Arc<DB>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            &db_shard,
            &S::shard_progress_key(shard_id),
            metadata_progress,
        )?;
        let myself = Self {
            shard_id,
            db_shard,
            _phantom: PhantomData,
        };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up {} shard {shard_id}.",
            S::name(),
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
            .iter_with_opts::<S::StaleIndexSchema>(read_opts)?;
        // Seek to the first stale-index row at or after `current_progress`.
        iter.seek(&current_progress)?;
        for item in iter {
            let (index, _) = item?;
            if index.stale_since_version > target_version {
                break;
            }
            batch.delete::<S::StaleIndexSchema>(&index)?;
            if !index.is_first_write() {
                batch.delete::<S::ValueSchema>(&(index.state_key_hash, index.version))?;
            }
        }
        batch.put::<DbMetadataSchema>(
            &S::shard_progress_key(self.shard_id),
            &DbMetadataValue::Version(target_version),
        )?;

        self.db_shard.write_schemas(batch)
    }

    pub(in crate::pruner) fn shard_id(&self) -> usize {
        self.shard_id
    }
}

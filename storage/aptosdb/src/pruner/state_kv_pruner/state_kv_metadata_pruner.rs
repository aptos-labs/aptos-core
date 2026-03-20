// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    state_kv_db::StateKvDb,
    utils::get_progress,
};
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::sync::Arc;

pub(in crate::pruner) struct StateKvMetadataPruner {
    state_kv_db: Arc<StateKvDb>,
}

impl StateKvMetadataPruner {
    pub(in crate::pruner) fn new(state_kv_db: Arc<StateKvDb>) -> Self {
        Self { state_kv_db }
    }

    /// Records pruning progress. The actual deletion of stale state values
    /// is handled by `StateKvShardPruner` per shard.
    pub(in crate::pruner) fn prune(&self, target_version: Version) -> Result<()> {
        let mut batch = SchemaBatch::new();

        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;

        self.state_kv_db.metadata_db().write_schemas(batch)
    }

    pub(in crate::pruner) fn progress(&self) -> Result<Version> {
        Ok(get_progress(
            self.state_kv_db.metadata_db(),
            &DbMetadataKey::StateKvPrunerProgress,
        )?
        .unwrap_or(0))
    }
}

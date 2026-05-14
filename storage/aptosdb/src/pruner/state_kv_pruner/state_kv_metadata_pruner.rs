// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    pruner::state_kv_pruner::generics::StateValuePrunerSchema,
    schema::db_metadata::{DbMetadataSchema, DbMetadataValue},
    utils::get_progress,
};
use aptos_schemadb::{batch::SchemaBatch, DB};
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::{marker::PhantomData, sync::Arc};

pub(in crate::pruner) struct StateKvMetadataPruner<S> {
    metadata_db: Arc<DB>,
    _phantom: PhantomData<S>,
}

impl<S: StateValuePrunerSchema> StateKvMetadataPruner<S> {
    pub(in crate::pruner) fn new(metadata_db: Arc<DB>) -> Self {
        Self {
            metadata_db,
            _phantom: PhantomData,
        }
    }

    /// Records pruning progress. The actual deletion of stale values is
    /// handled by `StateKvShardPruner` per shard.
    pub(in crate::pruner) fn prune(&self, target_version: Version) -> Result<()> {
        let mut batch = SchemaBatch::new();

        batch.put::<DbMetadataSchema>(
            &S::pruner_progress_key(),
            &DbMetadataValue::Version(target_version),
        )?;

        self.metadata_db.write_schemas(batch)
    }

    pub(in crate::pruner) fn progress(&self) -> Result<Version> {
        Ok(get_progress(&self.metadata_db, &S::pruner_progress_key())?.unwrap_or(0))
    }
}

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        stale_state_value_index::StaleStateValueIndexSchema,
        stale_state_value_index_by_key_hash::StaleStateValueIndexByKeyHashSchema,
        state_value::StateValueSchema,
    },
    state_kv_db::StateKvDb,
    utils::get_progress,
};
use velor_schemadb::batch::SchemaBatch;
use velor_storage_interface::Result;
use velor_types::transaction::Version;
use std::sync::Arc;

pub(in crate::pruner) struct StateKvMetadataPruner {
    state_kv_db: Arc<StateKvDb>,
}

impl StateKvMetadataPruner {
    pub(in crate::pruner) fn new(state_kv_db: Arc<StateKvDb>) -> Self {
        Self { state_kv_db }
    }

    pub(in crate::pruner) fn prune(
        &self,
        current_progress: Version,
        target_version: Version,
    ) -> Result<()> {
        let mut batch = SchemaBatch::new();

        if self.state_kv_db.enabled_sharding() {
            let num_shards = self.state_kv_db.num_shards();
            // NOTE: This can be done in parallel if it becomes the bottleneck.
            for shard_id in 0..num_shards {
                let mut iter = self
                    .state_kv_db
                    .db_shard(shard_id)
                    .iter::<StaleStateValueIndexByKeyHashSchema>()?;
                iter.seek(&current_progress)?;
                for item in iter {
                    let (index, _) = item?;
                    if index.stale_since_version > target_version {
                        break;
                    }
                }
            }
        } else {
            let mut iter = self
                .state_kv_db
                .metadata_db()
                .iter::<StaleStateValueIndexSchema>()?;
            iter.seek(&current_progress)?;
            for item in iter {
                let (index, _) = item?;
                if index.stale_since_version > target_version {
                    break;
                }
                batch.delete::<StaleStateValueIndexSchema>(&index)?;
                batch.delete::<StateValueSchema>(&(index.state_key, index.version))?;
            }
        }

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

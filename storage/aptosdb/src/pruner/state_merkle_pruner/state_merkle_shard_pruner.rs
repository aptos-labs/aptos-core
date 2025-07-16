// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pruner::{
        pruner_utils::get_or_initialize_subpruner_progress,
        state_merkle_pruner::{generics::StaleNodeIndexSchemaTrait, StateMerklePruner},
    },
    schema::{
        db_metadata::{DbMetadataSchema, DbMetadataValue},
        jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    },
};
use anyhow::Result;
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_logger::info;
use aptos_schemadb::{batch::SchemaBatch, schema::KeyCodec, DB};
use aptos_types::transaction::Version;
use std::{marker::PhantomData, sync::Arc};

pub(in crate::pruner) struct StateMerkleShardPruner<S> {
    shard_id: usize,
    db_shard: Arc<DB>,
    _phantom: PhantomData<S>,
}

impl<S: StaleNodeIndexSchemaTrait> StateMerkleShardPruner<S>
where
    StaleNodeIndex: KeyCodec<S>,
{
    pub(in crate::pruner) fn new(
        shard_id: usize,
        db_shard: Arc<DB>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            &db_shard,
            &S::progress_metadata_key(Some(shard_id)),
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
        loop {
            let mut batch = SchemaBatch::new();
            let (indices, next_version) = StateMerklePruner::get_stale_node_indices(
                &self.db_shard,
                current_progress,
                target_version,
            )?;

            indices.into_iter().try_for_each(|index| {
                batch.delete::<JellyfishMerkleNodeSchema>(&index.node_key)?;
                batch.delete::<S>(&index)
            })?;

            let mut done = true;
            if let Some(next_version) = next_version {
                if next_version <= target_version {
                    done = false;
                }
            }

            if done {
                batch.put::<DbMetadataSchema>(
                    &S::progress_metadata_key(Some(self.shard_id)),
                    &DbMetadataValue::Version(target_version),
                )?;
            }

            self.db_shard.write_schemas(batch)?;

            if done {
                break;
            }
        }

        Ok(())
    }

    pub(in crate::pruner) fn shard_id(&self) -> usize {
        self.shard_id
    }
}

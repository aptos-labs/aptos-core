// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pruner::state_merkle_pruner::{generics::StaleNodeIndexSchemaTrait, StateMerklePruner},
    schema::{
        db_metadata::{DbMetadataSchema, DbMetadataValue},
        jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    },
    utils::get_progress,
};
use anyhow::Result;
use velor_jellyfish_merkle::StaleNodeIndex;
use velor_schemadb::{batch::SchemaBatch, schema::KeyCodec, DB};
use velor_types::transaction::{AtomicVersion, Version};
use std::{
    cmp::max,
    marker::PhantomData,
    sync::{atomic::Ordering, Arc},
};

pub(in crate::pruner) struct StateMerkleMetadataPruner<S> {
    metadata_db: Arc<DB>,
    next_version: AtomicVersion,
    _phantom: PhantomData<S>,
}

impl<S: StaleNodeIndexSchemaTrait> StateMerkleMetadataPruner<S>
where
    StaleNodeIndex: KeyCodec<S>,
{
    pub(in crate::pruner) fn new(metadata_db: Arc<DB>) -> Self {
        Self {
            metadata_db,
            next_version: AtomicVersion::new(0),
            _phantom: PhantomData,
        }
    }

    pub(in crate::pruner) fn maybe_prune_single_version(
        &self,
        current_progress: Version,
        target_version: Version,
    ) -> Result<Option<Version>> {
        let next_version = self.next_version.load(Ordering::SeqCst);
        // This max here is only to handle the case when next version is not initialized.
        let target_version_for_this_round = max(next_version, current_progress);
        if target_version_for_this_round > target_version {
            return Ok(None);
        }

        // When next_version is not initialized, this call is used to initialize it.
        let (indices, next_version) = StateMerklePruner::get_stale_node_indices(
            &self.metadata_db,
            current_progress,
            target_version_for_this_round,
            usize::MAX,
        )?;

        let mut batch = SchemaBatch::new();
        indices.into_iter().try_for_each(|index| {
            batch.delete::<JellyfishMerkleNodeSchema>(&index.node_key)?;
            batch.delete::<S>(&index)
        })?;

        batch.put::<DbMetadataSchema>(
            &S::progress_metadata_key(None),
            &DbMetadataValue::Version(target_version_for_this_round),
        )?;

        self.metadata_db.write_schemas(batch)?;

        self.next_version
            // If next_version is None, meaning we've already reached the end of stale index.
            // Updating it to the target_version to make sure it's still making progress.
            .store(next_version.unwrap_or(target_version), Ordering::SeqCst);

        Ok(Some(target_version_for_this_round))
    }

    pub(in crate::pruner) fn progress(&self) -> Result<Version> {
        Ok(get_progress(&self.metadata_db, &S::progress_metadata_key(None))?.unwrap_or(0))
    }
}

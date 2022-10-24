// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::db_metadata::DbMetadataSchema;
use crate::pruner::state_store::generics::StaleNodeIndexSchemaTrait;
use crate::schema::db_metadata::DbMetadataValue;
use crate::{
    jellyfish_merkle_node::JellyfishMerkleNodeSchema, metrics::PRUNER_LEAST_READABLE_VERSION,
    pruner::db_pruner::DBPruner, pruner_utils, StaleNodeIndexCrossEpochSchema,
    OTHER_TIMERS_SECONDS,
};
use anyhow::Result;
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_logger::error;
use aptos_types::transaction::{AtomicVersion, Version};
use schemadb::schema::KeyCodec;
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::sync::{atomic::Ordering, Arc};

pub mod generics;
pub(crate) mod state_value_pruner;

#[cfg(test)]
mod test;

pub const STATE_MERKLE_PRUNER_NAME: &str = "state_merkle_pruner";

/// Responsible for pruning the state tree.
#[derive(Debug)]
pub struct StateMerklePruner<S> {
    /// State DB.
    state_merkle_db: Arc<DB>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    /// 1. min readable version
    /// 2. if things before that version fully cleaned
    progress: Mutex<(Version, bool)>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: StaleNodeIndexSchemaTrait> DBPruner for StateMerklePruner<S>
where
    StaleNodeIndex: KeyCodec<S>,
{
    fn name(&self) -> &'static str {
        STATE_MERKLE_PRUNER_NAME
    }

    fn prune(&self, batch_size: usize) -> Result<Version> {
        if !self.is_pruning_pending() {
            return Ok(self.min_readable_version());
        }
        let min_readable_version = self.min_readable_version();
        let target_version = self.target_version();

        match self.prune_state_merkle(min_readable_version, target_version, batch_size, None) {
            Ok(new_min_readable_version) => Ok(new_min_readable_version),
            Err(e) => {
                error!(
                    error = ?e,
                    "Error pruning stale states.",
                );
                Err(e)
                // On error, stop retrying vigorously by making next recv() blocking.
            }
        }
    }

    fn initialize_min_readable_version(&self) -> Result<Version> {
        Ok(self
            .state_merkle_db
            .get::<DbMetadataSchema>(&S::tag())?
            .map_or(0, |v| v.expect_version()))
    }

    fn min_readable_version(&self) -> Version {
        let (version, _) = *self.progress.lock();
        version
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::Relaxed);
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::Relaxed)
    }

    // used only by blanket `initialize()`, use the underlying implementation instead elsewhere.
    fn record_progress(&self, min_readable_version: Version) {
        self.record_progress_impl(min_readable_version, false /* is_fully_pruned */);
    }

    fn is_pruning_pending(&self) -> bool {
        let (min_readable_version, fully_pruned) = *self.progress.lock();
        self.target_version() > min_readable_version || !fully_pruned
    }

    /// (For tests only.) Updates the minimal readable version kept by pruner.
    fn testonly_update_min_version(&self, version: Version) {
        self.record_progress_impl(version, true /* is_fully_pruned */);
    }
}

impl<S: StaleNodeIndexSchemaTrait> StateMerklePruner<S>
where
    StaleNodeIndex: KeyCodec<S>,
{
    pub fn new(state_merkle_db: Arc<DB>) -> Self {
        let pruner = StateMerklePruner {
            state_merkle_db,
            target_version: AtomicVersion::new(0),
            progress: Mutex::new((0, true)),
            _phantom: std::marker::PhantomData,
        };
        pruner.initialize();
        pruner
    }

    // If the existing schema batch is not none, this function only adds items need to be
    // deleted to the schema batch and the caller is responsible for committing the schema batches
    // to the DB.
    pub fn prune_state_merkle(
        &self,
        min_readable_version: Version,
        target_version: Version,
        batch_size: usize,
        existing_schema_batch: Option<&mut SchemaBatch>,
    ) -> anyhow::Result<Version> {
        assert_ne!(batch_size, 0);
        let (indices, is_end_of_target_version) =
            self.get_stale_node_indices(min_readable_version, target_version, batch_size)?;
        if indices.is_empty() {
            self.record_progress_impl(target_version, is_end_of_target_version);
            Ok(target_version)
        } else {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["state_pruner_commit"])
                .start_timer();
            let new_min_readable_version =
                indices.last().expect("Should exist.").stale_since_version;

            // Delete stale nodes.
            if let Some(existing_schema_batch) = existing_schema_batch {
                indices.into_iter().try_for_each(|index| {
                    existing_schema_batch.delete::<JellyfishMerkleNodeSchema>(&index.node_key)?;
                    existing_schema_batch.delete::<S>(&index)
                })?;
            } else {
                let batch = SchemaBatch::new();
                indices.into_iter().try_for_each(|index| {
                    batch.delete::<JellyfishMerkleNodeSchema>(&index.node_key)?;
                    batch.delete::<S>(&index)
                })?;

                batch.put::<DbMetadataSchema>(
                    &S::tag(),
                    &DbMetadataValue::Version(new_min_readable_version),
                )?;

                // Commit to DB.
                self.state_merkle_db.write_schemas(batch)?;
            }

            // TODO(zcc): recording progress after writing schemas might provide wrong answers to
            // API calls when they query min_readable_version while the write_schemas are still in
            // progress.
            self.record_progress_impl(new_min_readable_version, is_end_of_target_version);
            Ok(new_min_readable_version)
        }
    }

    fn record_progress_impl(&self, min_readable_version: Version, is_fully_pruned: bool) {
        *self.progress.lock() = (min_readable_version, is_fully_pruned);
        PRUNER_LEAST_READABLE_VERSION
            .with_label_values(&[S::name()])
            .set(min_readable_version as i64);
    }

    fn get_stale_node_indices(
        &self,
        start_version: Version,
        target_version: Version,
        batch_size: usize,
    ) -> Result<(Vec<StaleNodeIndex>, bool)> {
        let mut indices = Vec::new();
        let mut iter = self.state_merkle_db.iter::<S>(ReadOptions::default())?;
        iter.seek(&StaleNodeIndex {
            stale_since_version: start_version,
            node_key: NodeKey::new_empty_path(0),
        })?;

        // over fetch by 1
        for _ in 0..=batch_size {
            if let Some((index, _)) = iter.next().transpose()? {
                if index.stale_since_version <= target_version {
                    indices.push(index);
                    continue;
                }
            }
            break;
        }

        let is_end_of_target_version = if indices.len() > batch_size {
            indices.pop();
            false
        } else {
            true
        };
        Ok((indices, is_end_of_target_version))
    }
}

impl StateMerklePruner<StaleNodeIndexCrossEpochSchema> {
    /// Prunes the genesis state and saves the db alterations to the given change set
    pub fn prune_genesis(state_merkle_db: Arc<DB>, batch: &mut SchemaBatch) -> Result<()> {
        let target_version = 1; // The genesis version is 0. Delete [0,1) (exclusive)
        let max_version = 1; // We should only be pruning a single version

        let state_pruner =
            pruner_utils::create_state_pruner::<StaleNodeIndexCrossEpochSchema>(state_merkle_db);
        state_pruner.set_target_version(target_version);

        let min_readable_version = state_pruner.min_readable_version();
        let target_version = state_pruner.target_version();
        state_pruner.prune_state_merkle(
            min_readable_version,
            target_version,
            max_version,
            Some(batch),
        )?;

        Ok(())
    }
}

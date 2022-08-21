// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::pruner::pruner_metadata::PrunerMetadata;
use crate::pruner::state_store::generics::StaleNodeIndexSchemaTrait;
use crate::pruner_metadata::PrunerMetadataSchema;
use crate::{
    jellyfish_merkle_node::JellyfishMerkleNodeSchema, metrics::PRUNER_LEAST_READABLE_VERSION,
    pruner::db_pruner::DBPruner, utils, StaleNodeIndexCrossEpochSchema, OTHER_TIMERS_SECONDS,
};
use anyhow::Result;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_logger::error;
use aptos_types::transaction::{AtomicVersion, Version};
use schemadb::schema::KeyCodec;
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

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
    min_readable_version: AtomicVersion,
    /// Keeps track of if the target version has been fully pruned to see if there is pruning
    /// pending.
    pruned_to_the_end_of_target_version: AtomicBool,
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
        let min_readable_version = self.min_readable_version.load(Ordering::Relaxed);
        let target_version = self.target_version();

        return match self.prune_state_merkle(min_readable_version, target_version, batch_size, None)
        {
            Ok(new_min_readable_version) => Ok(new_min_readable_version),
            Err(e) => {
                error!(
                    error = ?e,
                    "Error pruning stale states.",
                );
                Err(e)
                // On error, stop retrying vigorously by making next recv() blocking.
            }
        };
    }

    fn initialize_min_readable_version(&self) -> Result<Version> {
        Ok(self
            .state_merkle_db
            .get::<PrunerMetadataSchema>(&S::tag())?
            .map_or(0, |pruned_until_version| match pruned_until_version {
                PrunerMetadata::LatestVersion(version) => version,
            }))
    }

    fn min_readable_version(&self) -> Version {
        self.min_readable_version.load(Ordering::Relaxed)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::Relaxed);
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::Relaxed)
    }

    fn record_progress(&self, min_readable_version: Version) {
        self.min_readable_version
            .store(min_readable_version, Ordering::Relaxed);
        PRUNER_LEAST_READABLE_VERSION
            .with_label_values(&["state_store"])
            .set(min_readable_version as i64);
    }

    fn is_pruning_pending(&self) -> bool {
        self.target_version() > self.min_readable_version()
            || !self
                .pruned_to_the_end_of_target_version
                .load(Ordering::Relaxed)
    }

    /// (For tests only.) Updates the minimal readable version kept by pruner.
    fn testonly_update_min_version(&self, version: Version) {
        self.min_readable_version.store(version, Ordering::Relaxed)
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
            min_readable_version: AtomicVersion::new(0),
            pruned_to_the_end_of_target_version: AtomicBool::new(false),
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
            self.pruned_to_the_end_of_target_version
                .store(is_end_of_target_version, Ordering::Relaxed);
            self.record_progress(target_version);
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

                batch.put::<PrunerMetadataSchema>(
                    &S::tag(),
                    &PrunerMetadata::LatestVersion(new_min_readable_version),
                )?;

                // Commit to DB.
                self.state_merkle_db.write_schemas(batch)?;
            }

            // TODO(zcc): recording progress after writing schemas might provide wrong answers to
            // API calls when they query min_readable_version while the write_schemas are still in
            // progress.
            self.record_progress(new_min_readable_version);
            self.pruned_to_the_end_of_target_version
                .store(is_end_of_target_version, Ordering::Relaxed);
            Ok(new_min_readable_version)
        }
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
            utils::create_state_pruner::<StaleNodeIndexCrossEpochSchema>(state_merkle_db);
        state_pruner.set_target_version(target_version);

        let min_readable_version = state_pruner.min_readable_version.load(Ordering::Relaxed);
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

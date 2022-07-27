// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jellyfish_merkle_node::JellyfishMerkleNodeSchema, metrics::PRUNER_LEAST_READABLE_VERSION,
    pruner::db_pruner::DBPruner, stale_node_index::StaleNodeIndexSchema, OTHER_TIMERS_SECONDS,
};
use anyhow::Result;
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_logger::error;
use aptos_types::transaction::{AtomicVersion, Version};
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[cfg(test)]
mod test;

pub const STATE_STORE_PRUNER_NAME: &str = "state store pruner";

pub struct StateStorePruner {
    db: Arc<DB>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    min_readable_version: AtomicVersion,
    // Keeps track of if the target version has been fully pruned to see if there is pruning
    // pending.
    pruned_to_the_end_of_target_version: AtomicBool,
}

impl DBPruner for StateStorePruner {
    fn name(&self) -> &'static str {
        STATE_STORE_PRUNER_NAME
    }

    fn prune(&self, batch_size: usize) -> Result<Version> {
        if !self.is_pruning_pending() {
            return Ok(self.min_readable_version());
        }
        let min_readable_version = self.min_readable_version.load(Ordering::Relaxed);
        let target_version = self.target_version();

        return match self.prune_state_store(min_readable_version, target_version, batch_size) {
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
        let mut iter = self
            .db
            .iter::<StaleNodeIndexSchema>(ReadOptions::default())?;
        iter.seek_to_first();
        Ok(iter.next().transpose()?.map_or(0, |(index, _)| {
            index
                .stale_since_version
                .checked_sub(1)
                .expect("Nothing is stale since version 0.")
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
}

impl StateStorePruner {
    pub fn new(db: Arc<DB>) -> Self {
        let pruner = StateStorePruner {
            db,
            target_version: AtomicVersion::new(0),
            min_readable_version: AtomicVersion::new(0),
            pruned_to_the_end_of_target_version: AtomicBool::new(false),
        };
        pruner.initialize();
        pruner
    }

    pub fn prune_state_store(
        &self,
        min_readable_version: Version,
        target_version: Version,
        batch_size: usize,
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
            let batch = SchemaBatch::new();
            // Delete stale nodes.
            indices.into_iter().try_for_each(|index| {
                batch.delete::<JellyfishMerkleNodeSchema>(&index.node_key)?;
                batch.delete::<StaleNodeIndexSchema>(&index)
            })?;
            // Delete the stale node indices.
            self.db.write_schemas(batch)?;
            self.pruned_to_the_end_of_target_version
                .store(is_end_of_target_version, Ordering::Relaxed);
            self.record_progress(new_min_readable_version);
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
        let mut iter = self
            .db
            .iter::<StaleNodeIndexSchema>(ReadOptions::default())?;
        iter.seek(&start_version)?;

        let mut num_items = batch_size;
        while num_items > 0 {
            if let Some(item) = iter.next() {
                let (index, _) = item?;
                if index.stale_since_version > target_version {
                    return Ok((indices, /*is_end_of_target_version=*/ true));
                }
                num_items -= 1;
                indices.push(index);
            } else {
                // No more stale nodes.
                break;
            }
        }

        // This is to deal with the case where number of items reaches 0 but there are still
        // stale nodes in the indices.
        if let Some(next_item) = iter.next() {
            let (next_index, _) = next_item?;
            if next_index.stale_since_version > target_version {
                return Ok((indices, /*is_end_of_target_version=*/ true));
            }
        }

        // This is to deal with the case where we reaches the end of the indices regardless of
        // whether we have `num_items` in `indices`.
        let mut is_end_of_target_version = true;
        if let Some(last_index) = indices.last() {
            is_end_of_target_version = last_index.stale_since_version == target_version;
        }
        Ok((indices, is_end_of_target_version))
    }
}

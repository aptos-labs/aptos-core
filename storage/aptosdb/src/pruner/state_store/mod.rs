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
use std::sync::{atomic::Ordering, Arc};

#[cfg(test)]
mod test;

pub const STATE_STORE_PRUNER_NAME: &str = "state store pruner";

pub struct StateStorePruner {
    db: Arc<DB>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    min_readable_version: AtomicVersion,
    pruning_completed_version: AtomicVersion,
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
            Ok(new_min_readable_version) => {
                self.record_progress(new_min_readable_version);
                Ok(new_min_readable_version)
            }
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
        self.target_version() > self.pruning_completed_version.load(Ordering::Relaxed)
    }
}

impl StateStorePruner {
    pub fn new(db: Arc<DB>) -> Self {
        let pruner = StateStorePruner {
            db,
            target_version: AtomicVersion::new(0),
            min_readable_version: AtomicVersion::new(0),
            pruning_completed_version: AtomicVersion::new(0),
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
        let (indices, pruning_completed_version) =
            self.get_stale_node_indices(min_readable_version, target_version, batch_size)?;
        if indices.is_empty() {
            self.pruning_completed_version
                .store(pruning_completed_version, Ordering::Relaxed);
            Ok(target_version)
        } else {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["state_pruner_commit"])
                .start_timer();
            let new_min_readable_version =
                indices.last().expect("Should exist.").stale_since_version;
            let mut batch = SchemaBatch::new();
            // Delete stale nodes.
            indices.into_iter().try_for_each(|index| {
                batch.delete::<JellyfishMerkleNodeSchema>(&index.node_key)?;
                batch.delete::<StaleNodeIndexSchema>(&index)
            })?;
            // Delete the stale node indices.
            self.db.write_schemas(batch)?;
            self.pruning_completed_version
                .store(pruning_completed_version, Ordering::Relaxed);
            Ok(new_min_readable_version)
        }
    }

    // Return the stale node indices to prune in one iteration. It either returns `batch_size` of
    // items or all the remaining items whose stale_since_version = target version. It also returns
    // the last version which all the stale nodes has been pruned.
    fn get_stale_node_indices(
        &self,
        start_version: Version,
        target_version: Version,
        batch_size: usize,
    ) -> Result<(Vec<StaleNodeIndex>, Version)> {
        let mut indices = Vec::new();
        let mut iter = self
            .db
            .iter::<StaleNodeIndexSchema>(ReadOptions::default())?;
        iter.seek(&start_version)?;

        let mut num_items = batch_size;
        let mut pruning_completed_version = if start_version == 0 {
            0
        } else {
            start_version - 1
        };
        while num_items > 0 {
            if let Some(item) = iter.next() {
                match item {
                    Err(e) => {
                        return Err(e);
                    }
                    Ok((index, _)) => {
                        if index.stale_since_version > target_version {
                            pruning_completed_version = target_version;
                            break;
                        }
                        num_items -= 1;
                        indices.push(index);
                    }
                }
            } else {
                // No more stale nodes.
                break;
            }
        }

        if let Some(next_item) = iter.next() {
            match next_item {
                Err(e) => {
                    return Err(e);
                }
                Ok((next_item, _)) => {
                    if next_item.stale_since_version > target_version {
                        pruning_completed_version = target_version;
                    }
                }
            }
        } else {
            // No more stale nodes.
            if let Some(last_index) = indices.last() {
                pruning_completed_version = last_index.stale_since_version;
            } else {
                // No stale nodes between `start_version` and `target_version`.
                pruning_completed_version = target_version;
            }
        }
        Ok((indices, pruning_completed_version))
    }
}

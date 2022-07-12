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
use schemadb::{ReadOptions, SchemaBatch, SchemaIterator, DB};
use std::{
    iter::Peekable,
    sync::{atomic::Ordering, Arc},
};

#[cfg(test)]
mod test;

pub const STATE_STORE_PRUNER_NAME: &str = "state store pruner";

pub struct StateStorePruner {
    db: Arc<DB>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    min_readable_version: AtomicVersion,
}

impl DBPruner for StateStorePruner {
    fn name(&self) -> &'static str {
        STATE_STORE_PRUNER_NAME
    }

    fn prune(&self, _ledger_db_batch: &mut SchemaBatch, batch_size: u64) -> Result<Version> {
        if !self.is_pruning_pending() {
            return Ok(self.min_readable_version());
        }
        let min_readable_version = self.min_readable_version.load(Ordering::Relaxed);
        let target_version = self.target_version();
        return match self.prune_state_store(
            min_readable_version,
            target_version,
            batch_size as usize,
        ) {
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
}

impl StateStorePruner {
    pub fn new(db: Arc<DB>) -> Self {
        let pruner = StateStorePruner {
            db,
            target_version: AtomicVersion::new(0),
            min_readable_version: AtomicVersion::new(0),
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
        let indices_iter = StaleNodeIndicesByNumberOfItemsIterator::new(
            &self.db,
            min_readable_version,
            batch_size,
            target_version,
        )?;
        let indices = indices_iter.collect::<anyhow::Result<Vec<_>>>()?;

        if indices.is_empty() {
            Ok(min_readable_version)
        } else {
            let _timer = OTHER_TIMERS_SECONDS
                .with_label_values(&["pruner_commit"])
                .start_timer();
            let new_min_readable_version =
                indices.last().expect("Should exist.").stale_since_version;
            let mut batch = SchemaBatch::new();
            let mut index_batch = SchemaBatch::new();
            // Delete stale nodes.
            indices.into_iter().try_for_each(|index| {
                batch.delete::<JellyfishMerkleNodeSchema>(&index.node_key)?;
                index_batch.delete::<StaleNodeIndexSchema>(&index)
            })?;
            // Delete the stale node indices.
            self.db.write_schemas(batch)?;
            self.db.write_schemas(index_batch)?;
            Ok(new_min_readable_version)
        }
    }
}
// This iterator will traverse `num_items` of stale nodes or all the stale nodes whose
// `stale_since_version` = `target_min_readable_version`, whichever limit is reached first.
struct StaleNodeIndicesByNumberOfItemsIterator<'a> {
    inner: Peekable<SchemaIterator<'a, StaleNodeIndexSchema>>,
    num_items: usize,
    target_min_readable_version: Version,
}

impl<'a> StaleNodeIndicesByNumberOfItemsIterator<'a> {
    fn new(
        db: &'a DB,
        start_pruning_version: Version,
        num_items: usize,
        target_min_readable_version: Version,
    ) -> anyhow::Result<Self> {
        let mut iter = db.iter::<StaleNodeIndexSchema>(ReadOptions::default())?;
        iter.seek(&start_pruning_version)?;

        Ok(Self {
            inner: iter.peekable(),
            num_items,
            target_min_readable_version,
        })
    }

    fn next_result(&mut self) -> Result<Option<StaleNodeIndex>> {
        if self.num_items == 0 {
            return Ok(None);
        }
        match self.inner.next().transpose()? {
            None => {
                self.num_items -= 1;
                Ok(None)
            }
            Some((index, _)) => {
                if index.stale_since_version > self.target_min_readable_version {
                    return Ok(None);
                }
                self.num_items -= 1;
                Ok(Some(index))
            }
        }
    }
}

impl<'a> Iterator for StaleNodeIndicesByNumberOfItemsIterator<'a> {
    type Item = anyhow::Result<StaleNodeIndex>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_result().transpose()
    }
}

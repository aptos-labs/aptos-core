// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jellyfish_merkle_node::JellyfishMerkleNodeSchema, metrics::PRUNER_LEAST_READABLE_VERSION,
    pruner::db_pruner::DBPruner, stale_node_index::StaleNodeIndexSchema, OTHER_TIMERS_SECONDS,
};
use anyhow::Result;
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_logger::{error, warn};
use aptos_types::transaction::{AtomicVersion, Version};
use schemadb::{ReadOptions, SchemaBatch, SchemaIterator, DB};
use std::{
    iter::Peekable,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

#[cfg(test)]
mod test;

pub const STATE_STORE_PRUNER_NAME: &str = "state store pruner";

pub struct StateStorePruner {
    db: Arc<DB>,
    max_index_purged_version: AtomicVersion,
    index_purged_at: Mutex<Instant>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    min_readable_version: AtomicVersion,
}

impl DBPruner for StateStorePruner {
    fn name(&self) -> &'static str {
        STATE_STORE_PRUNER_NAME
    }

    fn prune(&self, _ledger_db_batch: &mut SchemaBatch, max_versions: u64) -> Result<Version> {
        if !self.is_pruning_pending() {
            return Ok(self.min_readable_version());
        }
        let min_readable_version = self.min_readable_version.load(Ordering::Relaxed);
        let target_version = self.target_version();
        return match prune_state_store(
            &self.db,
            min_readable_version,
            target_version,
            max_versions as usize,
        ) {
            Ok(new_min_readable_version) => {
                self.record_progress(new_min_readable_version);
                // Try to purge the log.
                if let Err(e) = self.maybe_purge_index() {
                    warn!(
                        error = ?e,
                        "Failed purging state node index, ignored.",
                    );
                }
                Ok(new_min_readable_version)
            }
            Err(e) => {
                error!(
                    error = ?e,
                    "Error pruning stale state nodes.",
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
    pub fn new(db: Arc<DB>, max_index_purged_version: Version, index_purged_at: Instant) -> Self {
        let pruner = StateStorePruner {
            db,
            max_index_purged_version: AtomicVersion::new(max_index_purged_version),
            index_purged_at: Mutex::new(index_purged_at),
            target_version: AtomicVersion::new(0),
            min_readable_version: AtomicVersion::new(0),
        };
        pruner.initialize();
        pruner
    }

    /// Purge the stale node index so that after restart not too much already pruned stuff is dealt
    /// with again (although no harm is done deleting those then non-existent things.)
    ///
    /// We issue (range) deletes on the index only periodically instead of after every pruning batch
    /// to avoid sending too many deletions to the DB, which takes disk space and slows it down.
    fn maybe_purge_index(&self) -> Result<()> {
        const MIN_INTERVAL: Duration = Duration::from_secs(10);
        const MIN_VERSIONS: u64 = 60000;

        let min_readable_version = self.min_readable_version.load(Ordering::Relaxed);
        let max_index_purged_version = self.max_index_purged_version.load(Ordering::Relaxed);

        // A deletion is issued at most once in one minute and when the pruner has progressed by at
        // least 60000 versions (assuming the pruner deletes as slow as 1000 versions per second,
        // this imposes at most one minute of work in vain after restarting.)
        let now = Instant::now();
        if now - *self.index_purged_at.lock() > MIN_INTERVAL
            && min_readable_version - max_index_purged_version > MIN_VERSIONS
        {
            self.db.range_delete::<StaleNodeIndexSchema, Version>(
                &(max_index_purged_version + 1), // begin is inclusive
                &(min_readable_version + 1),     // end is exclusive
            )?;
            // The index items at min_readable_version has been consumed already because they
            // indicate nodes that became stale at that version, keeping that version readable,
            // hence is purged above.
            self.max_index_purged_version
                .store(min_readable_version, Ordering::Relaxed);
            *self.index_purged_at.lock() = now;
        }
        Ok(())
    }

    pub fn target_version(&self) -> Version {
        self.target_version.load(Ordering::Relaxed)
    }
}

pub fn prune_state_store(
    db: &DB,
    min_readable_version: Version,
    target_version: Version,
    max_versions: usize,
) -> Result<Version> {
    let indices = StaleNodeIndicesByVersionIterator::new(db, min_readable_version, target_version)?
        .take(max_versions) // Iterator<Item = Result<Vec<StaleNodeIndex>>>
        .collect::<Result<Vec<_>>>()? // now Vec<Vec<StaleNodeIndex>>
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    if indices.is_empty() {
        Ok(min_readable_version)
    } else {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["pruner_commit"])
            .start_timer();
        let new_min_readable_version = indices.last().expect("Should exist.").stale_since_version;
        let mut batch = SchemaBatch::new();
        indices
            .into_iter()
            .try_for_each(|index| batch.delete::<JellyfishMerkleNodeSchema>(&index.node_key))?;
        db.write_schemas(batch)?;
        Ok(new_min_readable_version)
    }
}

struct StaleNodeIndicesByVersionIterator<'a> {
    inner: Peekable<SchemaIterator<'a, StaleNodeIndexSchema>>,
    target_min_readable_version: Version,
}

impl<'a> StaleNodeIndicesByVersionIterator<'a> {
    fn new(
        db: &'a DB,
        min_readable_version: Version,
        target_min_readable_version: Version,
    ) -> Result<Self> {
        let mut iter = db.iter::<StaleNodeIndexSchema>(ReadOptions::default())?;
        iter.seek(&min_readable_version)?;

        Ok(Self {
            inner: iter.peekable(),
            target_min_readable_version,
        })
    }

    fn next_result(&mut self) -> Result<Option<Vec<StaleNodeIndex>>> {
        match self.inner.next().transpose()? {
            None => Ok(None),
            Some((index, _)) => {
                let version = index.stale_since_version;
                if version > self.target_min_readable_version {
                    return Ok(None);
                }

                let mut indices = vec![index];
                while let Some(res) = self.inner.peek() {
                    if let Ok((index_ref, _)) = res {
                        if index_ref.stale_since_version != version {
                            break;
                        }
                    }

                    let (index, _) = self.inner.next().transpose()?.expect("Should be Some.");
                    indices.push(index);
                }

                Ok(Some(indices))
            }
        }
    }
}

impl<'a> Iterator for StaleNodeIndicesByVersionIterator<'a> {
    type Item = Result<Vec<StaleNodeIndex>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_result().transpose()
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Exact state reads at any version.
//!
//! State KV is allowed to keep only the last write to a key within each block, so a plain seek at
//! a version in the middle of a block can miss a write squashed by a later write in that block.
//! Reads here find the block first, and look at its write sets when the key is written later in
//! it. This also holds on rows written per version, so no knowledge of the layout is needed.

use crate::{db::AptosDB, metrics::COUNTER, pruner::PrunerManager};
use aptos_infallible::Mutex;
use aptos_metrics_core::IntCounterVecHelper;
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use lru::LruCache;
use std::{collections::BTreeMap, num::NonZeroUsize};

#[cfg(test)]
#[path = "exact_state_read_test.rs"]
mod exact_state_read_test;

pub(super) const MAX_CACHED_BLOCK_RANGES: usize = 10_000;

/// Caches ranges `[start, end]` with state checkpoints at `end` and at `start - 1` (unless `start`
/// is 0), evicting the least recently used.
pub(crate) struct BlockRangeCache {
    inner: Mutex<BlockRangeCacheInner>,
}

struct BlockRangeCacheInner {
    /// `start -> end`
    ranges: BTreeMap<Version, Version>,
    /// Recency of `ranges`, by start.
    lru: LruCache<Version, ()>,
}

impl BlockRangeCache {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(BlockRangeCacheInner {
                ranges: BTreeMap::new(),
                lru: LruCache::new(
                    NonZeroUsize::new(capacity).expect("Capacity must be positive."),
                ),
            }),
        }
    }

    /// Returns a cached range containing `version`.
    pub(crate) fn get(&self, version: Version) -> Option<(Version, Version)> {
        let mut inner = self.inner.lock();
        let (&start, &end) = inner.ranges.range(..=version).next_back()?;
        if version > end {
            return None;
        }
        inner.lru.promote(&start);
        Some((start, end))
    }

    pub(crate) fn insert(&self, start: Version, end: Version) {
        self.inner.lock().insert(start, end);
    }

    /// Inserts the blocks of a chunk starting at `first_version` and ending at
    /// `checkpoint_versions`.
    pub(crate) fn insert_blocks(&self, first_version: Version, checkpoint_versions: &[Version]) {
        let mut inner = self.inner.lock();
        let mut start = first_version;
        for &end in checkpoint_versions {
            inner.insert(start, end);
            start = end + 1;
        }
    }

    #[cfg(test)]
    pub(crate) fn clear(&self) {
        let mut inner = self.inner.lock();
        inner.ranges.clear();
        inner.lru.clear();
    }
}

impl BlockRangeCacheInner {
    fn insert(&mut self, start: Version, end: Version) {
        debug_assert!(start <= end, "Bad block range [{start}, {end}].");
        self.ranges.insert(start, end);
        if let Some((evicted, ())) = self.lru.push(start, ())
            && evicted != start
        {
            self.ranges.remove(&evicted);
        }
    }
}

impl AptosDB {
    /// Returns the value of `state_key` at `version`, which doesn't have to be a block end.
    pub(super) fn get_state_value_with_version_exact(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        match self.get_state_value_with_version_in_block(state_key, version) {
            // Reads at a block end don't need the rest of the block, which may be pruned (e.g.
            // right after a fast sync).
            Err(AptosDbError::LedgerPruned { .. }) if self.is_state_checkpoint(version)? => self
                .state_kv_db
                .get_state_value_with_version_by_version(state_key, version),
            result => result,
        }
    }

    fn get_state_value_with_version_in_block(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        let (start, end) = self.get_block_range(version)?;
        if version == end {
            return self
                .state_kv_db
                .get_state_value_with_version_by_version(state_key, version);
        }

        // Each block keeps a row for its last write to a key, so any write in `(version, end]`
        // leaves a row there.
        let row = match self
            .state_kv_db
            .get_state_value_row_by_version(state_key, end)?
        {
            // The last write at or before `version` may have lost its row to a later write in the
            // same block, which then started at or after `start`.
            Some((row_version, _)) if row_version > version => {
                let row = self
                    .state_kv_db
                    .get_state_value_row_by_version(state_key, version)?;
                let write = self.get_latest_write_in_range(state_key, start, version)?;
                row.into_iter()
                    .chain(write)
                    .max_by_key(|(version, _)| *version)
            },
            row => row,
        };
        Ok(row.and_then(|(version, value_opt)| value_opt.map(|value| (version, value))))
    }

    /// Returns a range `[start, end]` containing `version`, with state checkpoints at `end` and at
    /// `start - 1` (unless `start` is 0). Normally the block of `version`.
    fn get_block_range(&self, version: Version) -> Result<(Version, Version)> {
        if let Some(range) = self.block_range_cache.get(version) {
            return Ok(range);
        }

        COUNTER.inc_with(&["state_read_block_range_cache_miss"]);
        let (start, end) = match self.get_block_range_from_index(version)? {
            Some(range) => range,
            None => self.get_block_range_from_transaction_infos(version)?,
        };
        self.block_range_cache.insert(start, end);
        Ok((start, end))
    }

    /// The block index comes from `NewBlockEvent`s, which old history and some blocks (e.g. a
    /// genesis transaction) don't have, so it's only used if the checkpoints agree.
    fn get_block_range_from_index(&self, version: Version) -> Result<Option<(Version, Version)>> {
        let Some((start, next_start)) = self
            .ledger_db
            .metadata_db()
            .get_block_boundaries_by_version(version)?
        else {
            return Ok(None);
        };

        let end = match next_start {
            Some(next_start) => {
                let end = next_start - 1;
                if !self.is_state_checkpoint(end)? {
                    return Ok(None);
                }
                end
            },
            // Not known from the index for the latest block.
            None => self.get_checkpoint_version_at_or_after(version)?,
        };
        if start > 0 && !self.is_state_checkpoint(start - 1)? {
            return Ok(None);
        }
        Ok(Some((start, end)))
    }

    fn get_block_range_from_transaction_infos(
        &self,
        version: Version,
    ) -> Result<(Version, Version)> {
        COUNTER.inc_with(&["state_read_block_range_txn_info_scan"]);

        let end = self.get_checkpoint_version_at_or_after(version)?;
        let min_readable_version = self.ledger_pruner.get_min_readable_version();
        let start = match self
            .ledger_db
            .transaction_info_db()
            .get_checkpoint_version_before(version, min_readable_version)?
        {
            Some(checkpoint_version) => checkpoint_version + 1,
            None if min_readable_version == 0 => 0,
            None => {
                return Err(AptosDbError::LedgerPruned {
                    data_type: "TransactionInfo".to_string(),
                    version: min_readable_version - 1,
                    min_available_version: min_readable_version,
                })
            },
        };
        Ok((start, end))
    }

    fn is_state_checkpoint(&self, version: Version) -> Result<bool> {
        self.error_if_ledger_pruned("TransactionInfo", version)?;
        Ok(self
            .ledger_db
            .transaction_info_db()
            .get_transaction_info(version)?
            .has_state_checkpoint_hash())
    }

    fn get_checkpoint_version_at_or_after(&self, version: Version) -> Result<Version> {
        self.error_if_ledger_pruned("TransactionInfo", version)?;
        self.ledger_db
            .transaction_info_db()
            .get_checkpoint_version_at_or_after(version)?
            .ok_or_else(|| {
                AptosDbError::NotFound(format!("State checkpoint at or after version {version}"))
            })
    }

    /// Returns the version and value of the last write to `state_key` in `[start, version]`.
    /// `None` value means deletion.
    fn get_latest_write_in_range(
        &self,
        state_key: &StateKey,
        start: Version,
        version: Version,
    ) -> Result<Option<(Version, Option<StateValue>)>> {
        self.error_if_ledger_pruned("WriteSet", start)?;
        COUNTER.inc_with(&["state_read_write_set_scan"]);

        let write_sets = self
            .ledger_db
            .write_set_db()
            .get_write_set_iter(start, (version - start + 1) as usize)?;
        let mut latest = None;
        let mut next_version = start;
        for write_set in write_sets {
            if let Some(write_op) = write_set?.get_write_op(state_key) {
                latest = Some((next_version, write_op.as_state_value_opt().cloned()));
            }
            next_version += 1;
        }
        if next_version <= version {
            return Err(AptosDbError::NotFound(format!(
                "WriteSet at version {next_version}"
            )));
        }
        Ok(latest)
    }
}

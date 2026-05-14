// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Pruner for the sharded `position_value` CF.
//!
//! Drives off `stale_position_value_index`: rows whose
//! `stale_since_version <= pruning_horizon` are deleted, along with
//! their corresponding entry in the value CF. Stale-index entries
//! are partitioned across the same 16 shards as the values they
//! reference, so we walk each shard's stale-index in parallel.
//!
//! Runs independently of the main-state pruner so position history
//! can be trimmed on a tighter schedule.

#![forbid(unsafe_code)]

use crate::{
    position_db::PositionDb,
    position_metrics::POSITION_PRUNE_ROWS,
    schema::{
        position_value::PositionValueSchema,
        stale_position_value_index::StalePositionValueIndexSchema,
    },
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_schemadb::{batch::SchemaBatch, DB};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::transaction::Version;
use rayon::prelude::*;
use std::sync::Arc;

/// Stale-value pruner for sharded `position_db`.
#[derive(Debug)]
pub struct PositionPruner {
    position_db: Arc<PositionDb>,
}

impl PositionPruner {
    pub fn new(position_db: Arc<PositionDb>) -> Self {
        Self { position_db }
    }

    /// Prune all stale rows with `stale_since_version <= horizon`
    /// across every shard. Returns the total number of stale-index
    /// entries drained (one per logical superseded row).
    ///
    /// The two-step dance (index scan → batch delete of both the
    /// value row and the index row) keeps the index and value CF
    /// consistent even under concurrent writes within a shard: the
    /// index entry is only emitted AFTER the superseding write
    /// lands, and we delete the index row last in our batch, so a
    /// crash mid-prune leaves both rows present (the next prune
    /// cycle re-discovers them).
    pub fn prune_up_to(&self, horizon: Version) -> Result<usize> {
        // Run the per-shard fan-out on the dedicated background pool
        // so pruner work doesn't compete with foreground rayon
        // consumers (block executor, JMT batch puts, etc.). Matches
        // how `LedgerPruner::prune` schedules its sub-pruners.
        let totals: Vec<Result<usize>> = THREAD_MANAGER.get_background_pool().install(|| {
            self.position_db
                .shards()
                .par_iter()
                .map(|shard| prune_shard(shard, horizon))
                .collect()
        });
        let mut total = 0usize;
        for shard_total in totals {
            total += shard_total?;
        }
        if total > 0 {
            POSITION_PRUNE_ROWS.inc_by(total as u64);
        }
        Ok(total)
    }
}

fn prune_shard(shard: &Arc<DB>, horizon: Version) -> Result<usize> {
    let mut iter = shard.iter::<StalePositionValueIndexSchema>()?;
    iter.seek_to_first();
    let mut batch = SchemaBatch::new();
    let mut pruned = 0usize;
    for row in iter.by_ref() {
        let (idx_key, _) = row?;
        if idx_key.stale_since_version > horizon {
            break;
        }
        batch
            .delete::<PositionValueSchema>(&(idx_key.state_key_hash, idx_key.version))
            .map_err(|e| AptosDbError::Other(format!("position_value delete failed: {e}")))?;
        batch
            .delete::<StalePositionValueIndexSchema>(&idx_key)
            .map_err(|e| {
                AptosDbError::Other(format!("stale_position_value_index delete failed: {e}"))
            })?;
        pruned += 1;
    }
    if pruned > 0 {
        shard
            .write_schemas(batch)
            .map_err(|e| AptosDbError::Other(format!("position pruner commit failed: {e}")))?;
    }
    Ok(pruned)
}

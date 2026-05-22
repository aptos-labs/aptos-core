// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Background thread that writes `PositionMerkleCommit` batches into
//! `position_merkle_db`. Mirrors [`crate::state_store::state_merkle_batch_committer`]
//! — second stage of the two-stage async commit pipeline.
//!
//! The first-stage `position_snapshot_committer` walks the accumulated
//! position deltas through the sharded JMT pipeline
//! (`merklize_value_set_for_shard × 16 + calculate_top_levels`),
//! producing one `RawBatch` per shard plus one for the top-level
//! metadata. This thread hands that batch tuple straight to
//! `ShardedJmtMerkleDb::commit`, which writes the 16 shards in
//! parallel and commits the metadata DB last.

#![forbid(unsafe_code)]

use crate::{
    common::{run_batch_committer_loop, CommitMessage, MerkleBatch},
    metrics::OTHER_TIMERS_SECONDS,
    position_merkle_db::PositionMerkleDb,
};
use aptos_crypto::HashValue;
use aptos_logger::{info, trace};
use aptos_metrics_core::TimerHelper;
use aptos_types::transaction::Version;
use std::sync::{mpsc::Receiver, Arc};

/// Message handed off from the position snapshot-committer to this
/// thread. Mirrors `StateMerkleCommit` shape — pre-built `RawBatch`es
/// (in a shared [`MerkleBatch`]) ready for `ShardedJmtMerkleDb::commit`.
pub(crate) struct PositionMerkleCommit {
    /// JMT version this batch produced.
    pub version: Version,
    /// New position subtree root.
    pub root_hash: HashValue,
    /// Pre-built per-shard + top-level batches.
    pub batch: MerkleBatch,
}

pub(crate) struct PositionMerkleBatchCommitter {
    merkle_db: Arc<PositionMerkleDb>,
    receiver: Receiver<CommitMessage<PositionMerkleCommit>>,
}

impl PositionMerkleBatchCommitter {
    pub fn new(
        merkle_db: Arc<PositionMerkleDb>,
        receiver: Receiver<CommitMessage<PositionMerkleCommit>>,
    ) -> Self {
        Self {
            merkle_db,
            receiver,
        }
    }

    pub fn run(self) {
        let Self {
            merkle_db,
            receiver,
        } = self;
        run_batch_committer_loop(receiver, |commit| {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["position_batch_committer_work"]);
            let PositionMerkleCommit {
                version,
                root_hash,
                batch,
            } = commit;
            // `ShardedJmtMerkleDb::commit` handles version-cache eviction
            // internally — see `sharded_jmt_merkle_db.rs`.
            merkle_db
                .commit(version, batch.top_levels_batch, batch.batches_for_shards)
                .expect("Position merkle commit failed.");
            info!(
                version = version,
                root_hash = %root_hash,
                "Position merkle snapshot committed."
            );
        });
        trace!("Position merkle batch committing thread exit.");
    }
}

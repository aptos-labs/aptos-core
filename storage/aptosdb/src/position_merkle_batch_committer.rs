// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    common::{run_batch_committer_loop, CommitMessage, MerkleBatch},
    metrics::OTHER_TIMERS_SECONDS,
    position_buffered_state::{PositionPersistedState, PositionStateWithSummary},
    position_merkle_db::PositionMerkleDb,
    position_pruner::PositionPruner,
    pruner::PrunerManager,
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::{info, trace};
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::user_positions::UserPositions;
use aptos_types::transaction::Version;
use std::sync::{mpsc::Receiver, Arc};

pub(crate) struct PositionMerkleCommit {
    pub version: Version,
    pub root_hash: HashValue,
    pub batch: MerkleBatch,
    /// The real in-memory snapshot whose JMT nodes this batch persists.
    /// Published to `persisted` after the commit so the in-memory chain
    /// can rebase onto it (see [`PositionPersistedState`]).
    pub snapshot: PositionStateWithSummary,
}

pub(crate) struct PositionMerkleBatchCommitter {
    merkle_db: Arc<PositionMerkleDb>,
    receiver: Receiver<CommitMessage<PositionMerkleCommit>>,
    position_pruner: Arc<PositionPruner>,
    persisted: PositionPersistedState,
    /// Bundle handle for the scanner-side `UserPositions` index.
    /// After a snapshot lands the chain is collapsed in place so the
    /// in-memory depth doesn't grow without bound.
    user_positions: Arc<Mutex<UserPositions>>,
}

impl PositionMerkleBatchCommitter {
    pub fn new(
        merkle_db: Arc<PositionMerkleDb>,
        receiver: Receiver<CommitMessage<PositionMerkleCommit>>,
        position_pruner: Arc<PositionPruner>,
        persisted: PositionPersistedState,
        user_positions: Arc<Mutex<UserPositions>>,
    ) -> Self {
        Self {
            merkle_db,
            receiver,
            position_pruner,
            persisted,
            user_positions,
        }
    }

    pub fn run(self) {
        let Self {
            merkle_db,
            receiver,
            position_pruner,
            persisted,
            user_positions,
        } = self;
        run_batch_committer_loop(receiver, |commit| {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["position_batch_committer_work"]);
            let PositionMerkleCommit {
                version,
                root_hash,
                batch,
                snapshot,
            } = commit;
            merkle_db
                .commit(version, batch.top_levels_batch, batch.batches_for_shards)
                .expect("Position merkle commit failed.");
            info!(
                version = version,
                root_hash = %root_hash,
                "Position merkle snapshot committed."
            );
            // Activate the position merkle pruners now that a snapshot
            // has persisted.
            position_pruner
                .state_merkle_pruner
                .maybe_set_pruner_target_db_version(version);
            position_pruner
                .epoch_snapshot_pruner
                .maybe_set_pruner_target_db_version(version);
            // Advance the persisted base only now that the JMT nodes at
            // `version` are on disk, so cold-key proofs against this base
            // are serviceable.
            persisted.set(snapshot);
            // Collapse the scanner-side `UserPositions` chain into a
            // fresh family holding the current full state. Bounds
            // in-memory chain depth by snapshot cadence — outstanding
            // speculative branches keep the old family alive until
            // they drop, then it's collected.
            let mut up = user_positions.lock();
            *up = up.rebase();
        });
        trace!("Position merkle batch committing thread exit.");
    }
}

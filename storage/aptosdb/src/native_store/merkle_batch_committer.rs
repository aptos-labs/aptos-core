// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    common::{run_batch_committer_loop, CommitMessage, MerkleBatch},
    metrics::OTHER_TIMERS_SECONDS,
    native_store::{
        buffered_state::{NativePersistedState, NativeStateWithSummary},
        merkle_db::NativeMerkleDb,
        pruner::NativeStorePruner,
        NativeStoreSpec,
    },
    pruner::PrunerManager,
};
use aptos_crypto::HashValue;
use aptos_logger::{info, trace};
use aptos_metrics_core::TimerHelper;
use aptos_types::transaction::Version;
use std::sync::{mpsc::Receiver, Arc};

pub(crate) struct NativeMerkleCommit {
    pub version: Version,
    pub root_hash: HashValue,
    pub batch: MerkleBatch,
    /// The real in-memory snapshot whose JMT nodes this batch persists.
    /// Published to `persisted` after the commit so the in-memory chain
    /// can rebase onto it (see [`NativePersistedState`]).
    pub snapshot: NativeStateWithSummary,
}

pub(crate) struct NativeMerkleBatchCommitter<T: NativeStoreSpec> {
    merkle_db: Arc<NativeMerkleDb<T>>,
    receiver: Receiver<CommitMessage<NativeMerkleCommit>>,
    pruner: Arc<NativeStorePruner<T>>,
    persisted: NativePersistedState,
}

impl<T: NativeStoreSpec> NativeMerkleBatchCommitter<T> {
    pub fn new(
        merkle_db: Arc<NativeMerkleDb<T>>,
        receiver: Receiver<CommitMessage<NativeMerkleCommit>>,
        pruner: Arc<NativeStorePruner<T>>,
        persisted: NativePersistedState,
    ) -> Self {
        Self {
            merkle_db,
            receiver,
            pruner,
            persisted,
        }
    }

    pub fn run(self) {
        let Self {
            merkle_db,
            receiver,
            pruner,
            persisted,
        } = self;
        let timer_label = format!("{}_batch_committer_work", T::NAME);
        run_batch_committer_loop(receiver, |commit| {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&[&timer_label]);
            let NativeMerkleCommit {
                version,
                root_hash,
                batch,
                snapshot,
            } = commit;
            merkle_db
                .commit(version, batch.top_levels_batch, batch.batches_for_shards)
                .unwrap_or_else(|e| panic!("{} merkle commit failed: {e}", T::NAME));
            info!(
                version = version,
                root_hash = %root_hash,
                store = T::NAME,
                "Native merkle snapshot committed."
            );
            // Activate the merkle pruners now that a snapshot has persisted.
            pruner
                .state_merkle_pruner
                .maybe_set_pruner_target_db_version(version);
            pruner
                .epoch_snapshot_pruner
                .maybe_set_pruner_target_db_version(version);
            // Advance the persisted base only now that the JMT nodes at
            // `version` are on disk, so cold-key proofs against this base
            // are serviceable.
            persisted.set(snapshot);
        });
        trace!(
            store = T::NAME,
            "Native merkle batch committing thread exit."
        );
    }
}

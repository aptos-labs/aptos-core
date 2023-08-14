// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state snapshot committer running in background thread within StateStore.

use crate::{
    metrics::OTHER_TIMERS_SECONDS,
    state_store::{
        buffered_state::CommitMessage,
        state_merkle_batch_committer::{StateMerkleBatch, StateMerkleBatchCommitter},
        StateDb,
    },
    versioned_node_cache::VersionedNodeCache,
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::trace;
use aptos_scratchpad::SmtAncestors;
use aptos_storage_interface::{jmt_update_refs, jmt_updates, state_delta::StateDelta, Result};
use aptos_types::state_store::state_value::StateValue;
use rayon::prelude::*;
use static_assertions::const_assert;
use std::{
    sync::{
        mpsc,
        mpsc::{Receiver, SyncSender},
        Arc,
    },
    thread::JoinHandle,
};

pub(crate) struct StateSnapshotCommitter {
    state_db: Arc<StateDb>,
    state_snapshot_commit_receiver: Receiver<CommitMessage<Arc<StateDelta>>>,
    state_merkle_batch_commit_sender: SyncSender<CommitMessage<StateMerkleBatch>>,
    join_handle: Option<JoinHandle<()>>,
}

impl StateSnapshotCommitter {
    const CHANNEL_SIZE: usize = 0;

    pub fn new(
        state_db: Arc<StateDb>,
        state_snapshot_commit_receiver: Receiver<CommitMessage<Arc<StateDelta>>>,
        smt_ancestors: SmtAncestors<StateValue>,
    ) -> Self {
        // Note: This is to ensure we cache nodes in memory from previous batches before they get committed to DB.
        const_assert!(
            StateSnapshotCommitter::CHANNEL_SIZE < VersionedNodeCache::NUM_VERSIONS_TO_CACHE
        );
        // Rendezvous channel
        let (state_merkle_batch_commit_sender, state_merkle_batch_commit_receiver) =
            mpsc::sync_channel(Self::CHANNEL_SIZE);
        let arc_state_db = Arc::clone(&state_db);
        let join_handle = std::thread::Builder::new()
            .name("state_batch_committer".to_string())
            .spawn(move || {
                let committer = StateMerkleBatchCommitter::new(
                    arc_state_db,
                    state_merkle_batch_commit_receiver,
                    smt_ancestors,
                );
                committer.run();
            })
            .expect("Failed to spawn state merkle batch committer thread.");
        Self {
            state_db,
            state_snapshot_commit_receiver,
            state_merkle_batch_commit_sender,
            join_handle: Some(join_handle),
        }
    }

    pub fn run(self) {
        while let Ok(msg) = self.state_snapshot_commit_receiver.recv() {
            match msg {
                CommitMessage::Data(delta_to_commit) => {
                    let version = delta_to_commit.current_version.expect("Cannot be empty");
                    let base_version = delta_to_commit.base_version;
                    let previous_epoch_ending_version = self
                        .state_db
                        .get_previous_epoch_ending(version)
                        .unwrap()
                        .map(|(v, _e)| v);

                    let (shard_root_nodes, batches_for_shards) = {
                        let _timer = OTHER_TIMERS_SECONDS
                            .with_label_values(&["calculate_batches_for_shards"])
                            .start_timer();

                        let shard_persisted_versions = self
                            .state_db
                            .state_merkle_db
                            .get_shard_persisted_versions(base_version)
                            .unwrap();

                        THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                            (0..16)
                                .into_par_iter()
                                .map(|shard_id| {
                                    let node_hashes = delta_to_commit
                                        .current
                                        .new_node_hashes_since(&delta_to_commit.base, shard_id);
                                    self.state_db.state_merkle_db.merklize_value_set_for_shard(
                                        shard_id,
                                        jmt_update_refs(&jmt_updates(
                                            &delta_to_commit.updates_since_base[shard_id as usize]
                                                .iter()
                                                .map(|(k, v)| (k, v.as_ref()))
                                                .collect(),
                                        )),
                                        Some(&node_hashes),
                                        version,
                                        base_version,
                                        shard_persisted_versions[shard_id as usize],
                                        previous_epoch_ending_version,
                                    )
                                })
                                .collect::<Result<Vec<_>>>()
                                .expect("Error calculating StateMerkleBatch for shards.")
                                .into_iter()
                                .unzip()
                        })
                    };

                    let (root_hash, top_levels_batch) = {
                        let _timer = OTHER_TIMERS_SECONDS
                            .with_label_values(&["calculate_top_levels_batch"])
                            .start_timer();
                        self.state_db
                            .state_merkle_db
                            .calculate_top_levels(
                                shard_root_nodes,
                                version,
                                base_version,
                                previous_epoch_ending_version,
                            )
                            .expect("Error calculating StateMerkleBatch for top levels.")
                    };

                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Data(StateMerkleBatch {
                            top_levels_batch,
                            batches_for_shards,
                            root_hash,
                            state_delta: delta_to_commit,
                        }))
                        .unwrap();
                },
                CommitMessage::Sync(finish_sender) => {
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Sync(finish_sender))
                        .unwrap();
                },
                CommitMessage::Exit => {
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Exit)
                        .unwrap();
                    break;
                },
            }
            trace!("State snapshot committing thread exit.")
        }
    }
}

impl Drop for StateSnapshotCommitter {
    fn drop(&mut self) {
        self.join_handle
            .take()
            .expect("state merkle batch commit thread must exist.")
            .join()
            .expect("state merkle batch thread should join peacefully.");
    }
}

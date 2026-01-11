// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This file defines the state snapshot committer running in background thread within StateStore.

use crate::{
    metrics::OTHER_TIMERS_SECONDS,
    state_merkle_db::StateMerkleDb,
    state_store::{
        buffered_state::CommitMessage,
        persisted_state::PersistedState,
        state_merkle_batch_committer::{
            StateMerkleBatch, StateMerkleBatchCommitter, StateMerkleCommit,
        },
        StateDb,
    },
    versioned_node_cache::VersionedNodeCache,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::SparseMerkleTree;
use aptos_storage_interface::{
    jmt_update_refs, state_store::state_with_summary::StateWithSummary, Result,
};
use aptos_types::{
    state_store::{hot_state::HotStateItem, state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
};
use rayon::prelude::*;
use static_assertions::const_assert;
use std::{
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread::JoinHandle,
};

pub(crate) struct StateSnapshotCommitter {
    state_db: Arc<StateDb>,
    /// Last snapshot merklized and sent for persistence, not guaranteed to have committed already.
    last_snapshot: StateWithSummary,
    state_snapshot_commit_receiver: Receiver<CommitMessage<StateWithSummary>>,
    state_merkle_batch_commit_sender: SyncSender<CommitMessage<StateMerkleCommit>>,
    join_handle: Option<JoinHandle<()>>,
}

impl StateSnapshotCommitter {
    const CHANNEL_SIZE: usize = 0;

    pub fn new(
        state_db: Arc<StateDb>,
        state_snapshot_commit_receiver: Receiver<CommitMessage<StateWithSummary>>,
        last_snapshot: StateWithSummary,
        persisted_state: PersistedState,
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
                    persisted_state.clone(),
                );
                committer.run();
            })
            .expect("Failed to spawn state merkle batch committer thread.");
        Self {
            state_db,
            last_snapshot,
            state_snapshot_commit_receiver,
            state_merkle_batch_commit_sender,
            join_handle: Some(join_handle),
        }
    }

    pub fn run(mut self) {
        while let Ok(msg) = self.state_snapshot_commit_receiver.recv() {
            match msg {
                CommitMessage::Data(snapshot) => {
                    let version = snapshot.version().expect("Cannot be empty");
                    let base_version = self.last_snapshot.version();
                    let previous_epoch_ending_version = self
                        .state_db
                        .ledger_db
                        .metadata_db()
                        .get_previous_epoch_ending(version)
                        .unwrap()
                        .map(|(v, _e)| v);
                    let min_version = self.last_snapshot.next_version();

                    // Element format: (key_hash, Option<(value_hash, key)>)
                    let (hot_updates, all_updates): (Vec<_>, Vec<_>) = snapshot
                        .make_delta(&self.last_snapshot)
                        .shards
                        .iter()
                        .map(|updates| {
                            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["hash_jmt_updates"]);
                            let mut hot_updates = Vec::new();
                            let mut all_updates = Vec::new();
                            for (key, slot) in updates.iter() {
                                if slot.is_hot() {
                                    hot_updates.push((
                                        CryptoHash::hash(&key),
                                        Some((
                                            HotStateItem::from(slot.clone()).hash(),
                                            key.clone(),
                                        )),
                                    ));
                                } else {
                                    hot_updates.push((CryptoHash::hash(&key), None));
                                }
                                if let Some(value) = slot.maybe_update_jmt(key, min_version) {
                                    all_updates.push(value);
                                }
                            }
                            (hot_updates, all_updates)
                        })
                        .unzip();

                    // TODO(HotState): for now we use `is_descendant_of` to determine if hot state
                    // summary is computed at all. When it's not enabled everything is
                    // `SparseMerkleTree::new_empty()`.
                    let hot_state_merkle_batch_opt = if snapshot
                        .summary()
                        .hot_state_summary
                        .is_descendant_of(&self.last_snapshot.summary().hot_state_summary)
                    {
                        self.state_db.hot_state_merkle_db.as_ref().map(|db| {
                            Self::merklize(
                                db,
                                base_version,
                                version,
                                &self.last_snapshot.summary().hot_state_summary,
                                &snapshot.summary().hot_state_summary,
                                hot_updates.try_into().expect("Must be 16 shards."),
                                previous_epoch_ending_version,
                            )
                            .expect("Failed to compute JMT commit batch for hot state.")
                            .0
                        })
                    } else {
                        // TODO(HotState): this means that the relevant code path isn't enabled yet.
                        None
                    };
                    let (state_merkle_batch, leaf_count) = Self::merklize(
                        &self.state_db.state_merkle_db,
                        base_version,
                        version,
                        &self.last_snapshot.summary().global_state_summary,
                        &snapshot.summary().global_state_summary,
                        all_updates.try_into().expect("Must be 16 shards."),
                        previous_epoch_ending_version,
                    )
                    .expect("Failed to compute JMT commit batch.");
                    let usage = snapshot.state().usage();
                    if !usage.is_untracked() {
                        assert_eq!(
                            leaf_count,
                            usage.items(),
                            "Num of state items mismatch: jmt: {}, state: {}",
                            leaf_count,
                            usage.items(),
                        );
                    }

                    self.last_snapshot = snapshot.clone();

                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Data(StateMerkleCommit {
                            snapshot,
                            hot_batch: hot_state_merkle_batch_opt,
                            cold_batch: state_merkle_batch,
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
        }
        info!("State snapshot committing thread exit.");
    }

    fn merklize(
        db: &StateMerkleDb,
        base_version: Option<Version>,
        version: Version,
        last_smt: &SparseMerkleTree,
        smt: &SparseMerkleTree,
        all_updates: [Vec<(HashValue, Option<(HashValue, StateKey)>)>; NUM_STATE_SHARDS],
        previous_epoch_ending_version: Option<Version>,
    ) -> Result<(StateMerkleBatch, usize)> {
        let shard_persisted_versions = db.get_shard_persisted_versions(base_version)?;

        let (shard_root_nodes, batches_for_shards) =
            THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                let _timer = OTHER_TIMERS_SECONDS.timer_with(&["calculate_batches_for_shards"]);
                all_updates
                    .par_iter()
                    .enumerate()
                    .map(|(shard_id, updates)| {
                        let node_hashes = smt.new_node_hashes_since(last_smt, shard_id as u8);
                        db.merklize_value_set_for_shard(
                            shard_id,
                            jmt_update_refs(updates),
                            Some(&node_hashes),
                            version,
                            base_version,
                            shard_persisted_versions[shard_id],
                            previous_epoch_ending_version,
                        )
                    })
                    .collect::<Result<Vec<_>>>()
                    .expect("Error calculating StateMerkleBatch for shards.")
                    .into_iter()
                    .unzip()
            });

        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["calculate_top_levels_batch"]);
        let (root_hash, leaf_count, top_levels_batch) = db.calculate_top_levels(
            shard_root_nodes,
            version,
            base_version,
            previous_epoch_ending_version,
        )?;
        assert_eq!(
            root_hash,
            smt.root_hash(),
            "root hash mismatch: jmt: {}, smt: {}",
            root_hash,
            smt.root_hash()
        );

        Ok((
            StateMerkleBatch {
                top_levels_batch,
                batches_for_shards,
            },
            leaf_count,
        ))
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

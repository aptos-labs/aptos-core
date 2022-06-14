// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use executor_types::StateSnapshotDelta;
use scratchpad::SparseMerkleTree;
use std::{
    collections::VecDeque,
    sync::{mpsc, mpsc::TryRecvError, Arc},
    time,
};
use storage_interface::DbWriter;

const NUM_COMMITTED_SMTS_TO_CACHE: usize = 5;
const NUM_MIN_COMMITS_TO_BATCH: usize = 20;

pub struct StateCommitter {
    commit_receiver: mpsc::Receiver<StateSnapshotDelta>,
    db: Arc<dyn DbWriter>,

    // keep some recently committed SMTs in mem as a naive cache
    cache_queue: VecDeque<SparseMerkleTree<StateValue>>,
    version: Version,
    smt: SparseMerkleTree<StateValue>,
    committed_smt: SparseMerkleTree<StateValue>,
    updates: Vec<(HashValue, (HashValue, StateKey))>,
    num_pending_commits: usize,
}

impl StateCommitter {
    pub fn new(
        commit_receiver: mpsc::Receiver<StateSnapshotDelta>,
        db: Arc<dyn DbWriter>,
        committed_smt: SparseMerkleTree<StateValue>,
    ) -> Self {
        let mut cache_queue = VecDeque::new();
        cache_queue.push_back(committed_smt.clone());

        Self {
            commit_receiver,
            db,

            cache_queue,
            version: 0,
            smt: committed_smt.clone(),
            committed_smt,
            updates: Vec::new(),
            num_pending_commits: 0,
        }
    }

    pub fn run(mut self) {
        loop {
            match self.commit_receiver.try_recv() {
                Ok(StateSnapshotDelta {
                    version,
                    smt,
                    jmt_updates,
                }) => {
                    self.version = version;
                    self.smt = smt;
                    self.updates.extend(jmt_updates.into_iter());
                    self.num_pending_commits += 1;
                }
                Err(TryRecvError::Empty) => {
                    if self.num_pending_commits < NUM_MIN_COMMITS_TO_BATCH {
                        std::thread::sleep(time::Duration::from_secs(1));
                    } else {
                        self.commit();
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    println!("Final state commit...");
                    self.commit();
                    return;
                }
            }
        }
    }

    fn commit(&mut self) {
        // commit
        info!(
            num_pending_commits = self.num_pending_commits,
            version = self.version,
            "Committing state.",
        );
        let mut to_commit = Vec::new();
        std::mem::swap(&mut to_commit, &mut self.updates);
        let node_hashes = self
            .smt
            .clone()
            .freeze()
            .new_node_hashes_since(&self.committed_smt.clone().freeze());
        self.db
            .save_state_snapshot(to_commit, Some(&node_hashes), self.version)
            .unwrap();
        info!("Committing state. Saved.");

        // reset pending updates
        self.num_pending_commits = 0;
        self.committed_smt = self.smt.clone();

        // cache maintenance
        if self.cache_queue.len() >= NUM_COMMITTED_SMTS_TO_CACHE {
            self.cache_queue.pop_front();
        }
        self.cache_queue.push_back(self.smt.clone());
    }
}

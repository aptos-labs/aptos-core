// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::info;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use scratchpad::SparseMerkleTree;
use std::{
    collections::{HashMap, VecDeque},
    sync::{mpsc, Arc},
};
use storage_interface::DbWriter;

const NUM_COMMITTED_SMTS_TO_CACHE: usize = 10;
const NUM_COMMITS_TO_BATCH: usize = 20;

pub struct StateCommitter {
    commit_receiver: mpsc::Receiver<(
        Version,
        SparseMerkleTree<StateValue>,
        Vec<HashMap<StateKey, StateValue>>,
    )>,
    db: Arc<dyn DbWriter>,
}

impl StateCommitter {
    pub fn new(
        commit_receiver: mpsc::Receiver<(
            Version,
            SparseMerkleTree<StateValue>,
            Vec<HashMap<StateKey, StateValue>>,
        )>,
        db: Arc<dyn DbWriter>,
    ) -> Self {
        Self {
            commit_receiver,
            db,
        }
    }

    pub fn run(self, base_smt: SparseMerkleTree<StateValue>) {
        // keep some recently committed SMTs in mem as a naive cache
        let mut cache_queue = VecDeque::with_capacity(NUM_COMMITTED_SMTS_TO_CACHE);
        cache_queue.push_back(base_smt.clone());
        // information wrt the next commit
        let mut version_to_commit;
        let mut smt_to_commit;
        let mut state_updates_to_commit = HashMap::new();
        let mut num_pending_commits = 0;
        let mut committed_smt = base_smt;

        while let Ok((version, smt, state_updates)) = self.commit_receiver.recv() {
            version_to_commit = version;
            smt_to_commit = smt;
            state_updates_to_commit.extend(state_updates.into_iter().flatten());
            num_pending_commits += 1;

            if num_pending_commits >= NUM_COMMITS_TO_BATCH {
                let mut to_commit = HashMap::new();
                std::mem::swap(&mut to_commit, &mut state_updates_to_commit);
                let node_hashes = smt_to_commit
                    .clone()
                    .freeze()
                    .new_node_hashes_since(&committed_smt.freeze());

                self.db
                    .merklize_state(to_commit, node_hashes, version_to_commit)
                    .unwrap();
                info!(version = version_to_commit, "State committed.");
                num_pending_commits = 0;
                committed_smt = smt_to_commit.clone();
                if cache_queue.len() >= NUM_COMMITTED_SMTS_TO_CACHE {
                    cache_queue.pop_front();
                }
                cache_queue.push_back(smt_to_commit);
            }
        }
    }
}

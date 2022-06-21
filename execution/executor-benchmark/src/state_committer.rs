// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_value::StateValue;
use aptosdb::state_store::{StateStore, TreeUpdateBatch};
use scratchpad::SparseMerkleTree;
use std::{
    collections::VecDeque,
    sync::{mpsc, Arc},
};

const NUM_COMMITTED_SMTS_TO_CACHE: usize = 5;

pub struct StateCommitter {
    commit_receiver: mpsc::Receiver<(SparseMerkleTree<StateValue>, TreeUpdateBatch)>,
    store: Arc<StateStore>,
    // keep some recently committed SMTs in mem as a naive cache
    cache_queue: VecDeque<SparseMerkleTree<StateValue>>,
}

impl StateCommitter {
    pub fn new(
        commit_receiver: mpsc::Receiver<(SparseMerkleTree<StateValue>, TreeUpdateBatch)>,
        store: Arc<StateStore>,
    ) -> Self {
        let cache_queue = VecDeque::with_capacity(NUM_COMMITTED_SMTS_TO_CACHE);

        Self {
            commit_receiver,
            store,
            cache_queue,
        }
    }

    pub fn run(mut self) {
        while let Ok((smt, batch)) = self.commit_receiver.recv() {
            self.store.persist_tree_update_batch(batch).unwrap();
            if self.cache_queue.len() >= NUM_COMMITTED_SMTS_TO_CACHE {
                self.cache_queue.pop_front();
            }
            self.cache_queue.push_back(smt);
        }
    }
}

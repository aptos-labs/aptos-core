// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use aptosdb::state_store::{StateStore, TreeUpdateBatch};
use executor_types::StateSnapshotDelta;
use scratchpad::SparseMerkleTree;
use std::{
    sync::{mpsc, mpsc::TryRecvError, Arc},
    time,
};
use storage_interface::jmt_update_refs;

const NUM_MIN_COMMITS_TO_BATCH: usize = 20;

pub struct StateCommitMaker {
    delta_receiver: mpsc::Receiver<StateSnapshotDelta>,
    state_commit_sender: mpsc::SyncSender<(SparseMerkleTree<StateValue>, TreeUpdateBatch)>,
    store: Arc<StateStore>,

    version: Version,
    smt: SparseMerkleTree<StateValue>,
    committed_version: Option<Version>,
    committed_smt: SparseMerkleTree<StateValue>,
    updates: Vec<(HashValue, (HashValue, StateKey))>,
    num_pending_commits: usize,
}

impl StateCommitMaker {
    pub fn new(
        delta_receiver: mpsc::Receiver<StateSnapshotDelta>,
        state_commit_sender: mpsc::SyncSender<(SparseMerkleTree<StateValue>, TreeUpdateBatch)>,
        store: Arc<StateStore>,
        committed_smt: SparseMerkleTree<StateValue>,
        committed_version: Option<Version>,
    ) -> Self {
        Self {
            delta_receiver,
            state_commit_sender,
            store,

            version: 0,
            smt: committed_smt.clone(),
            committed_smt,
            committed_version,
            updates: Vec::new(),
            num_pending_commits: 0,
        }
    }

    pub fn run(mut self) {
        loop {
            match self.delta_receiver.try_recv() {
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
                        self.make_state_commit();
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    println!("Final state commit...");
                    self.make_state_commit();
                    return;
                }
            }
        }
    }

    fn make_state_commit(&mut self) {
        // commit
        info!(
            num_pending_commits = self.num_pending_commits,
            version = self.version,
            "Making state commit.",
        );
        let mut to_commit = Vec::new();
        std::mem::swap(&mut to_commit, &mut self.updates);
        let node_hashes = self
            .smt
            .clone()
            .freeze()
            .new_node_hashes_since(&self.committed_smt.clone().freeze());
        let (_hash, batch) = self
            .store
            .merklize_value_set(
                jmt_update_refs(&to_commit),
                Some(&node_hashes),
                self.version,
                self.committed_version,
            )
            .unwrap();

        self.state_commit_sender
            .send((self.committed_smt.clone(), batch))
            .unwrap();

        // reset pending updates
        self.num_pending_commits = 0;
        self.committed_smt = self.smt.clone();
        self.committed_version = Some(self.version);
    }
}

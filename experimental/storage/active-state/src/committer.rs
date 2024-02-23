// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::CommitMessage;
use aptos_db::state_merkle_db::StateMerkleDb;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_jellyfish_merkle::node_type::Node;
use aptos_logger::info;
use aptos_schemadb::SchemaBatch;
use aptos_scratchpad::SparseMerkleTree;
use aptos_storage_interface::{jmt_update_refs, jmt_updates, Result};
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue, ShardedStateUpdates};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::{mpsc::Receiver, Arc};

pub struct ActionCommitter {
    version: u64,
    last_smt: Option<SparseMerkleTree<StateValue>>,
    state_merkle_db: Arc<StateMerkleDb>,
    receiver: Receiver<CommitMessage>,
}

impl ActionCommitter {
    pub fn new(
        state_merkle_db: Arc<StateMerkleDb>,
        receiver: Receiver<CommitMessage>,
        last_smt: Option<SparseMerkleTree<StateValue>>,
    ) -> Self {
        Self {
            version: 0,
            last_smt,
            state_merkle_db,
            receiver,
        }
    }

    pub fn run(&mut self) {
        loop {
            let commit_msg = self.receiver.recv().expect("Failure in receiving smts");
            if commit_msg.updates.is_empty() {
                break;
            }
            if commit_msg.smt.is_some() {
                self.commit_jmt_updates(commit_msg);
            } else {
                // ast, do nothing
                info!("No commit needed for AST")
            }
        }
    }

    fn generate_jmt_updates(
        &self,
        kvs: Vec<(StateKey, Option<StateValue>)>,
    ) -> ShardedStateUpdates {
        let mut updates_since_last = ShardedStateUpdates::default();
        kvs.into_iter().for_each(|(k, v)| {
            updates_since_last[k.get_shard_id() as usize].insert(k, v);
        });
        updates_since_last
    }

    fn commit_jmt_updates(&mut self, commit_msg: CommitMessage) {
        let sharded_updates = self.generate_jmt_updates(commit_msg.updates);
        let db = self.state_merkle_db.clone();
        let version = self.version;
        let tree = self.last_smt.as_ref().unwrap();
        let (shard_root_nodes, batches_for_shards): (Vec<Node<StateKey>>, Vec<SchemaBatch>) = {
            THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                (0..16u8)
                    .into_par_iter()
                    .map(|shard_id| {
                        let node_hashes = commit_msg
                            .smt
                            .as_ref()
                            .unwrap()
                            .new_node_hashes_since(tree, shard_id);
                        db.merklize_value_set_for_shard(
                            shard_id,
                            jmt_update_refs(&jmt_updates(
                                &sharded_updates[shard_id as usize]
                                    .iter()
                                    .map(|(k, v)| (k, v.as_ref()))
                                    .collect(),
                            )),
                            Some(&node_hashes),
                            version + 1,
                            Some(version),
                            Some(version),
                            Some(version),
                        )
                    })
                    .collect::<Result<Vec<_>>>()
                    .expect("Error calculating StateMerkleBatch for shards.")
                    .into_iter()
                    .unzip()
            })
        };

        // calculate the top levels batch
        let (_root_hash, top_levels_batch) = {
            self.state_merkle_db
                .calculate_top_levels(
                    shard_root_nodes,
                    self.version + 1,
                    Some(self.version),
                    Some(self.version),
                )
                .expect("Error calculating StateMerkleBatch for top levels.")
        };

        self.state_merkle_db
            .commit(self.version + 1, top_levels_batch, batches_for_shards)
            .unwrap();
        self.version += 1;
        self.last_smt = Some(commit_msg.smt.unwrap());
    }
}

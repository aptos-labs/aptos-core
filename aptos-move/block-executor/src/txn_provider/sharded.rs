// Copyright © Aptos Foundation

use std::fmt::Debug;
use aptos_mvhashmap::types::TxnIndex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::slice::Iter;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use dashmap::DashSet;
use serde::Serialize;
use aptos_logger::info;
use aptos_mvhashmap::MVHashMap;
use aptos_types::block_executor::partitioner::PartitionV3;
use aptos_types::executable::Executable;
use crate::scheduler::Scheduler;
use crate::task::{ExecutionStatus, Transaction, TransactionOutput};
use crate::txn_last_input_output::{TxnLastInputOutput, TxnOutput};
use crate::txn_provider::{BlockSTMPlugin, TxnIndexProvider};

pub enum CrossShardMessage<TO: TransactionOutput, TE: Debug> {
    Commit(CrossShardCommit<TO, TE>),
    Shutdown,
}

pub struct CrossShardCommit<TO: TransactionOutput, TE: Debug> {
    pub global_txn_idx: TxnIndex,
    pub txn_output: Arc<TxnOutput<TO, TE>>, //TODO: get rid of Arc so it can work with cross-machine sharding.
}

pub trait CrossShardClientForV3<TO: TransactionOutput, TE: Debug>: Send + Sync {
    fn send(&self, shard_idx: usize, output: CrossShardMessage<TO, TE>);
    fn recv(&self) -> CrossShardMessage<TO, TE>;
}

/// A BlockSTM plug-in that allows distributed execution with multiple BlockSTM instances.
pub struct ShardedTxnProvider<T: Transaction, TO: TransactionOutput, TE: Debug> {
    pub block_id: [u8; 32],
    pub num_shards: usize,
    pub shard_idx: usize,

    /// Maps a local txn idx to the txn itself.
    pub txns: Vec<T>,
    /// Maps a global txn idx to its shard and in-shard txn idx.
    pub local_idxs_by_global: HashMap<TxnIndex, usize>,
    /// Maps a local txn idx to its global idx.
    pub global_idxs: Vec<TxnIndex>,

    /// Maps a remote txn to its write set that we need to wait for locally.
    pub remote_dependencies: HashMap<TxnIndex, HashSet<T::Key>>,

    /// Maps a local txn to every shard that contain at least 1 follower.
    pub follower_shard_sets: Vec<Vec<usize>>,

    remote_committed_txns: DashSet<TxnIndex>,
    cross_shard_client: Arc<dyn CrossShardClientForV3<TO, TE>>,
}

impl<TX, TO, TE> ShardedTxnProvider<TX, TO, TE>
    where
        TX: Transaction,
        TO: TransactionOutput<Txn = TX>,
        TE: Debug + Send + Clone,
{
    pub fn new(
        block_id: [u8; 32],
        num_shards: usize,
        shard_idx: usize,
        cross_shard_client: Arc<dyn CrossShardClientForV3<TO, TE>>,
        txns: Vec<TX>,
        global_idxs: Vec<TxnIndex>,
        remote_dependencies: HashMap<TxnIndex, HashSet<TX::Key>>,
        follower_shard_sets: Vec<Vec<usize>>,
    ) -> Self {
        let local_idxs_by_global = global_idxs.iter().enumerate().map(|(local_idx, &global_idx)| (global_idx, local_idx)).collect();
        Self {
            block_id,
            num_shards,
            shard_idx,
            txns,
            local_idxs_by_global,
            global_idxs,
            remote_dependencies,
            follower_shard_sets,
            remote_committed_txns: Default::default(),
            cross_shard_client,
        }
    }

    pub fn txn(&self, idx: TxnIndex) -> &TX {
        let local_rank = self.local_idxs_by_global.get(&idx).copied().unwrap();
        &self.txns[local_rank]
    }

    pub fn global_idx_from_local(&self, local_idx: TxnIndex) -> TxnIndex {
        self.global_idxs[local_idx as usize]
    }

    pub fn num_local_txns(&self) -> usize {
        self.txns.len()
    }

    fn apply_updates_to_mv<TAG, X>(
        &self,
        versioned_cache: &MVHashMap<TX::Key, TAG, TX::Value, X>,
        global_txn_idx: TxnIndex,
        output: &TO
    ) where
        TAG: Hash + Clone + Eq + PartialEq + Debug + Serialize,
        X: Executable + 'static,
    {
        // First, apply writes.
        let write_version = (global_txn_idx, 0);
        for (k, v) in output
            .resource_write_set()
            .into_iter()
            .chain(output.aggregator_v1_write_set().into_iter())
        {
            versioned_cache.data().write(k, global_txn_idx, 0, v);
        }

        for (k, v) in output.module_write_set().into_iter() {
            versioned_cache.modules().write(k, global_txn_idx, v);
        }

        // Then, apply deltas.
        for (k, d) in output.aggregator_v1_delta_set().into_iter() {
            versioned_cache.data().add_delta(k, global_txn_idx, d);
        }
    }
}

impl<TX, TO, TE> TxnIndexProvider for ShardedTxnProvider<TX, TO, TE>
    where
        TX: Transaction,
        TO: TransactionOutput<Txn = TX>,
        TE: Debug + Send + Clone,
{
    fn end_txn_idx(&self) -> TxnIndex {
        TxnIndex::MAX
    }

    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn first_txn(&self) -> TxnIndex {
        self.global_idxs.first().copied().unwrap_or(self.end_txn_idx())
    }

    fn next_txn(&self, idx: TxnIndex) -> TxnIndex {
        if idx == self.end_txn_idx() {
            self.end_txn_idx()
        } else {
            let local_rank = self.local_idxs_by_global.get(&idx).copied().unwrap();
            self.global_idxs.get(local_rank + 1).copied().unwrap_or(self.end_txn_idx())
        }
    }

    fn txns(&self) -> Vec<TxnIndex> {
        self.global_idxs.clone()
    }

    fn txns_and_deps(&self) -> Vec<TxnIndex> {
        let x = self.global_idxs.iter();
        let y = self.remote_dependencies.keys();
        x.chain(y).copied().collect::<BTreeSet<_>>().into_iter().collect()
    }

    fn local_index(&self, idx: TxnIndex) -> usize {
        self.local_idxs_by_global.get(&idx).copied().unwrap()
    }

    fn is_local(&self, idx: TxnIndex) -> bool {
        self.local_idxs_by_global.contains_key(&idx)
    }

    fn txn_output_has_arrived(&self, txn_idx: TxnIndex) -> bool {
        self.remote_committed_txns.contains(&txn_idx)
    }

    fn block_idx(&self) -> &[u8] {
        self.block_id.as_slice()
    }

    fn shard_idx(&self) -> usize {
        self.shard_idx
    }
}


impl<TX, TO, TE> BlockSTMPlugin<TX, TO, TE> for ShardedTxnProvider<TX, TO, TE>
    where
        TX: Transaction,
        TO: TransactionOutput<Txn = TX>,
        TE: Debug + Send + Clone,
{
    fn remote_dependencies(&self) -> Vec<(TxnIndex, TX::Key)> {
        self.remote_dependencies.iter()
            .flat_map(|(txn_idx, keys)|{
                keys.iter().map(|key|(*txn_idx, key.clone()))
            })
            .collect()
    }

    fn run_sharding_msg_loop<TAG, X>(
        &self,
        mv: &MVHashMap<TX::Key, TAG, TX::Value, X>,
        scheduler: &Scheduler<Self>
    ) where
        TAG: Hash + Clone + Eq + PartialEq + Debug + Serialize,
        X: Executable + 'static
    {
        loop {
            match self.cross_shard_client.recv() {
                CrossShardMessage::Commit(CrossShardCommit{ global_txn_idx, txn_output }) => {
                    match txn_output.output_status() {
                        ExecutionStatus::Success(output) => {
                            self.apply_updates_to_mv(mv, global_txn_idx, output);
                        }
                        ExecutionStatus::SkipRest(output) => {
                            self.apply_updates_to_mv(mv, global_txn_idx, output);
                        }
                        ExecutionStatus::Abort(_) => {
                            //sharding todo: anything to do here?
                        }
                    }
                    self.remote_committed_txns.insert(global_txn_idx);
                    scheduler.fast_resume_dependents(global_txn_idx);
                },
                CrossShardMessage::Shutdown => break,
            }
        }
    }

    fn shutdown_receiver(&self) {
        self.cross_shard_client.send(self.shard_idx, CrossShardMessage::Shutdown);
    }

    fn txn(&self, idx: TxnIndex) -> &TX {
        let local_rank = self.local_idxs_by_global.get(&idx).copied().unwrap();
        &self.txns[local_rank]
    }

    fn on_local_commit(&self, txn_idx: TxnIndex, txn_output: Arc<TxnOutput<TO, TE>>) {
        let txn_local_index = self.local_index(txn_idx);
        for &shard_id in &self.follower_shard_sets[txn_local_index] {
            self.cross_shard_client.send(
                shard_id,
                CrossShardMessage::Commit(CrossShardCommit{ global_txn_idx: txn_idx, txn_output: txn_output.clone() }));
        }
    }

    fn use_dedicated_committing_thread(&self) -> bool {
        true
    }
}

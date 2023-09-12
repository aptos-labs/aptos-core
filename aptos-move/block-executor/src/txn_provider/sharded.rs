// Copyright © Aptos Foundation

use std::fmt::Debug;
use aptos_mvhashmap::types::TxnIndex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::slice::Iter;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use dashmap::DashSet;
use aptos_logger::info;
use aptos_mvhashmap::MVHashMap;
use aptos_types::executable::Executable;
use crate::task::{ExecutionStatus, Transaction, TransactionOutput};
use crate::txn_last_input_output::{TxnLastInputOutput, TxnOutput};
use crate::txn_provider::{RemoteCommit, ShardingMsg, TxnProviderTrait1, TxnProviderTrait2};

pub struct ShardedTxnProvider<T: Transaction, TO: TransactionOutput, TE: Debug> {
    pub block_id: u8,
    pub num_shards: usize,
    pub shard_id: usize,
    pub rx: Arc<Mutex<Receiver<ShardingMsg<TO, TE>>>>,
    pub senders: Vec<Mutex<Sender<ShardingMsg<TO, TE>>>>,

    /// Maps a local txn idx to the txn itself.
    pub txns: Vec<T>,
    /// Maps a global txn idx to its shard and in-shard txn idx.
    pub local_idxs_by_global: HashMap<TxnIndex, usize>,
    /// Maps a local txn idx to its global idx.
    pub global_idxs: Vec<TxnIndex>,

    /// Maps a remote txn to its write set that we need to wait for locally.
    pub remote_dependencies: HashMap<TxnIndex, HashSet<T::Key>>,

    /// Maps a local txn to every shard that contain at least 1 follower.
    pub following_shard_sets: Vec<Vec<usize>>,

    remote_committed_txns: DashSet<TxnIndex>,
}

impl<TX, TO, TE> ShardedTxnProvider<TX, TO, TE>
where
    TX: Transaction,
    TO: TransactionOutput<Txn = TX>,
    TE: Debug + Send + Clone,
{
    pub fn new(
        block_id: u8,
        num_shards: usize,
        shard_id: usize,
        rx: Arc<Mutex<Receiver<ShardingMsg<TO, TE>>>>,
        senders: Vec<Mutex<Sender<ShardingMsg<TO, TE>>>>,
        txns: Vec<TX>,
        local_idxs_by_global: HashMap<TxnIndex, usize>,
        global_idxs: Vec<TxnIndex>,
        remote_dependencies: HashMap<TxnIndex, HashSet<TX::Key>>,
        following_shard_sets: Vec<Vec<usize>>,
    ) -> Self {
        Self {
            block_id,
            num_shards,
            shard_id,
            rx,
            senders,
            txns,
            local_idxs_by_global,
            global_idxs,
            remote_dependencies,
            following_shard_sets,
            remote_committed_txns: Default::default(),
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

    pub fn local_rank(&self, idx: TxnIndex) -> usize {
        self.local_idxs_by_global.get(&idx).copied().unwrap() as usize
    }


    fn apply_updates_to_mv<X: Executable + 'static>(
        &self,
        versioned_cache: &MVHashMap<TX::Key, TX::Value, X>,
        global_txn_idx: TxnIndex,
        output: &TO
    ) {
        // First, apply writes.
        let write_version = (global_txn_idx, 0);
        for (k, v) in output
            .resource_write_set()
            .into_iter()
            .chain(output.aggregator_v1_write_set().into_iter())
        {
            versioned_cache.data().write(k, write_version, v);
        }

        for (k, v) in output.module_write_set().into_iter() {
            versioned_cache.modules().write(k, global_txn_idx, v);
        }

        // Then, apply deltas.
        for (k, d) in output.aggregator_v1_delta_set().into_iter() {
            versioned_cache.add_delta(k, global_txn_idx, d);
        }
    }
}

impl<TX, TO, TE> TxnProviderTrait1 for ShardedTxnProvider<TX, TO, TE>
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
        let local_rank = self.local_idxs_by_global.get(&idx).copied().unwrap();
        self.global_idxs.get(local_rank + 1).copied().unwrap_or(self.end_txn_idx())
    }

    fn txns(&self) -> Vec<TxnIndex> {
        self.global_idxs.clone()
    }

    fn txns_and_deps(&self) -> Vec<TxnIndex> {
        let x = self.global_idxs.iter();
        let y = self.remote_dependencies.keys();
        x.chain(y).copied().collect::<BTreeSet<_>>().into_iter().collect()
    }

    fn local_rank(&self, idx: TxnIndex) -> usize {
        self.local_idxs_by_global.get(&idx).copied().unwrap()
    }

    fn is_local(&self, idx: TxnIndex) -> bool {
        self.local_idxs_by_global.contains_key(&idx)
    }

    fn txn_output_has_arrived(&self, txn_idx: TxnIndex) -> bool {
        self.remote_committed_txns.contains(&txn_idx)
    }

    fn block_idx(&self) -> u8 {
        self.block_id
    }

    fn shard_idx(&self) -> usize {
        self.shard_id
    }
}


impl<TX, TO, TE> TxnProviderTrait2<TX, TO, TE> for ShardedTxnProvider<TX, TO, TE>
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

    fn run_sharding_msg_loop<X: Executable + 'static>(&self, mv: &MVHashMap<TX::Key, TX::Value, X>) {
        let listener = self.rx.lock().unwrap();
        loop {
            let msg = listener.recv().unwrap();
            match msg {
                ShardingMsg::RemoteCommit(msg) => {
                    let RemoteCommit { global_txn_idx, txn_output } = msg;
                    info!("block={}, shard={}, op=recv, txn_received={}.", self.block_id, self.shard_id, global_txn_idx);
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
                    //sharding todo: notify those who were blocked, and anything else?
                },
                ShardingMsg::Shutdown => {
                    break;
                },
            }
        }
    }

    fn shutdown_receiver(&self) {
        self.senders[self.shard_id].lock().unwrap().send(ShardingMsg::Shutdown).unwrap();
    }

    fn txn(&self, idx: TxnIndex) -> &TX {
        let local_rank = self.local_idxs_by_global.get(&idx).copied().unwrap();
        &self.txns[local_rank]
    }

    fn on_local_commit(&self, txn_idx: TxnIndex, txn_output: Arc<TxnOutput<TO, TE>>) {
        info!("block={}, shard={}, op=send, txn_to_be_sent={}", self.block_id, self.shard_id, txn_idx);
        let txn_local_rank = self.local_rank(txn_idx);
        for &shard_id in &self.following_shard_sets[txn_local_rank] {
            let msg = ShardingMsg::RemoteCommit(RemoteCommit {
                global_txn_idx: txn_idx,
                txn_output: txn_output.clone(),
            });
            let sender = self.senders[shard_id].lock().unwrap();
            sender.send(msg).unwrap();
        }
    }
}

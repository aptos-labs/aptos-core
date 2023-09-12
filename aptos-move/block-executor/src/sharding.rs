// Copyright © Aptos Foundation

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use dashmap::DashSet;
use rayon::Scope;
use aptos_logger::info;
use aptos_mvhashmap::MVHashMap;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::block_executor::partitioner::ShardId;
use aptos_types::executable::Executable;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::write_set::WriteOp;
use crate::errors::Error;
use crate::index_mapping::IndexHelper;
use crate::task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput};
use crate::txn_last_input_output::{TxnLastInputOutput, TxnOutput};

#[derive(Clone, Debug)]
pub struct RemoteCommit<TO: TransactionOutput, TE: Debug> {
    pub global_txn_idx: TxnIndex,
    pub txn_output: Arc<TxnOutput<TO, TE>>,
}

pub enum ShardingMsg<TO: TransactionOutput, TE: Debug> {
    RemoteCommit(RemoteCommit<TO, TE>),
    Shutdown,
}

pub struct TxnProvider<T: Transaction, TO: TransactionOutput, TE: Debug> {
    pub block_id: u8,
    pub sharding_mode: bool,
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

    index_helper: Arc<IndexHelper>,
}

impl<TX, TO, TE> TxnProvider<TX, TO, TE>
where
    TX: Transaction,
    TO: TransactionOutput<Txn = TX>,
    TE: Debug + Send + Clone,
{
    pub fn new_unsharded(txns: Vec<TX>) -> Self {
        let num_txns = txns.len();
        let indices: Vec<TxnIndex> = (0..(num_txns as TxnIndex)).collect();
        let local_rank_by_txn: HashMap<TxnIndex, usize> = (0..num_txns).map(|x|(x as TxnIndex, x)).collect();
        let (tx, rx) = mpsc::channel();
        let index_helper = IndexHelper {
            txns: Arc::new(indices.clone()),
            local_rank_by_txn: local_rank_by_txn.clone(),
            txns_and_deps: indices.clone(),
        };
        Self {
            block_id: 0,
            sharding_mode: false,
            num_shards: 1,
            shard_id: 0,
            rx: Arc::new(Mutex::new(rx)),
            senders: vec![Mutex::new(tx)],
            txns,
            local_idxs_by_global: local_rank_by_txn,
            global_idxs: indices,
            remote_dependencies: Default::default(),
            following_shard_sets: vec![vec![]; num_txns],
            index_helper: Arc::new(index_helper),
        }
    }

    pub fn new_sharded(
        block_id: u8,
        sharding_mode: bool,
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
        let index_helper = IndexHelper {
            txns: Arc::new(global_idxs.clone()),
            local_rank_by_txn: local_idxs_by_global.clone(),
            txns_and_deps: global_idxs.iter().copied().chain(remote_dependencies.keys().copied()).collect(),
        };

        Self {
            block_id,
            sharding_mode,
            num_shards,
            shard_id,
            rx,
            senders,
            txns,
            local_idxs_by_global,
            global_idxs,
            remote_dependencies,
            following_shard_sets,
            index_helper: Arc::new(index_helper),
        }
    }

    pub fn txn(&self, idx: TxnIndex) -> &TX {
        let local_rank = self.local_idxs_by_global.get(&idx).copied().unwrap();
        &self.txns[local_rank]
    }

    pub fn index_helper(&self) -> Arc<IndexHelper> {
        let ret = IndexHelper {
            txns: Arc::new(self.global_idxs.clone()),
            local_rank_by_txn: self.local_idxs_by_global.clone(),
            txns_and_deps: self.global_idxs.iter().copied().chain(self.remote_dependencies.keys().copied()).collect(),
        };
        Arc::new(ret)
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

    pub fn run_sharding_msg_loop<X: Executable + 'static>(&self, mv: &MVHashMap<TX::Key, TX::Value, X>) {
        if !self.sharding_mode { return; }
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
                    //sharding todo: notify those who were blocked, and anything else?
                },
                ShardingMsg::Shutdown => {
                    break;
                },
            }
        }
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

    pub fn shutdown_receiver(&self) {
        self.senders[self.shard_id].lock().unwrap().send(ShardingMsg::Shutdown).unwrap();
    }

    pub fn on_local_commit(
        &self,
        txn_idx: TxnIndex,
        txn_last_io: &TxnLastInputOutput<TX::Key, TO, TE>
    ) {
        if !self.sharding_mode { return; }

        info!("block={}, shard={}, op=send, txn_to_be_sent={}", self.block_id, self.shard_id, txn_idx);
        let txn_output = txn_last_io.txn_output(txn_idx).unwrap();
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

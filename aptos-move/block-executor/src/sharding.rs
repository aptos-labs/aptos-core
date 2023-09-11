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
use crate::task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput};
use crate::txn_last_input_output::{TxnLastInputOutput, TxnOutput};

pub type LocalTxnIndex = TxnIndex;
pub type GlobalTxnIndex = TxnIndex;

#[derive(Clone, Debug)]
pub struct RemoteCommit<TO: TransactionOutput, TE: Debug> {
    pub global_txn_idx: GlobalTxnIndex,
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
    pub local_idxs_by_global: HashMap<GlobalTxnIndex, LocalTxnIndex>,
    /// Maps a local txn idx to its global idx.
    pub global_idxs: Vec<TxnIndex>,

    /// Maps a local txn idx to the number of remote txns it still waits for.
    pub missing_dep_counts: Vec<Arc<(Mutex<usize>, Condvar)>>,

    /// Maps a global txn idx to its followers.
    pub follower_sets: Vec<BTreeSet<(ShardId, GlobalTxnIndex)>>,
}

impl<TX, TO, TE> TxnProvider<TX, TO, TE>
where
    TX: Transaction,
    TO: TransactionOutput<Txn = TX>,
    TE: Debug + Send + Clone,
{
    pub fn new_unsharded(txns: Vec<TX>) -> Self {
        let (tx, rx) = mpsc::channel();
        let num_txns = txns.len();
        Self {
            block_id: 0,
            sharding_mode: false,
            num_shards: 1,
            shard_id: 0,
            rx: Arc::new(Mutex::new(rx)),
            senders: vec![Mutex::new(tx)],
            txns,
            local_idxs_by_global: HashMap::new(),
            global_idxs: (0..(num_txns as TxnIndex)).collect(),
            missing_dep_counts: (0..num_txns).map(|idx|Arc::new((Mutex::new(0), Condvar::new()))).collect(),
            follower_sets: vec![],
        }
    }

    pub fn txn(&self, idx: LocalTxnIndex) -> &TX {
        &self.txns[idx as usize]
    }

    pub fn global_idx_from_local(&self, local_idx: TxnIndex) -> TxnIndex {
        self.global_idxs[local_idx as usize]
    }

    pub fn num_local_txns(&self) -> usize {
        self.txns.len()
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
                            // Nothing to do?
                        }
                    }

                    let cur_shard_followers = self.follower_sets[global_txn_idx as usize].range((self.shard_id, 0)..(self.shard_id + 1, 0));
                    for &(_shard_id, follower_global_idx) in cur_shard_followers {
                        info!("block={}, shard={}, op=recv, txn_received={}, txn_affected={}", self.block_id, self.shard_id, global_txn_idx, follower_global_idx);
                        let follower_local_idx = *self.local_idxs_by_global.get(&follower_global_idx).unwrap();
                        let (dep_counter_mutex, cvar) = &*self.missing_dep_counts[follower_local_idx as usize].clone();
                        let mut counter = dep_counter_mutex.lock().unwrap();
                        *counter -= 1;
                        info!("block={}, op=recv, cur_shard={}, txn_received={}, txn_affected={}, new_dep_count={}", self.block_id, self.shard_id, global_txn_idx, follower_global_idx, *counter);
                        if (*counter == 0) {
                            cvar.notify_all();
                        }
                    }
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
        global_txn_idx: GlobalTxnIndex,
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
        txn_idx: LocalTxnIndex,
        txn_last_io: &TxnLastInputOutput<TX::Key, TO, TE>
    ) {
        if !self.sharding_mode { return; }

        let global_idx = self.global_idx_from_local(txn_idx);
        info!("block={}, shard={}, op=send, txn_to_be_sent={}", self.block_id, self.shard_id, global_idx);
        let txn_output = txn_last_io.txn_output(txn_idx).unwrap();
        for shard_id in 0..self.num_shards {
            if shard_id == self.shard_id {
                continue;
            }
            if self.follower_sets[global_idx as usize].range((shard_id, 0)..(shard_id+1, 0)).next().is_some() {
                self.senders[shard_id].lock().unwrap().send(ShardingMsg::RemoteCommit(RemoteCommit { global_txn_idx: global_idx, txn_output: txn_output.clone() })).unwrap();
                info!("block={}, shard={}, op=send, txn_sent={}, dst_shard={}", self.block_id, self.shard_id, global_idx, shard_id);
            }
        }
    }

    pub fn wait_for_remote_deps(&self, idx: LocalTxnIndex) {
        if !self.sharding_mode { return; }
        let global_idx = self.global_idxs[idx as usize];
        let (count_guard, cvar) = &*self.missing_dep_counts[idx as usize];
        let mut count = count_guard.lock().unwrap();
        loop {
            let val = *count;
            info!("block={}, shard={}, txn_waiting={}, remote_deps_remaining={}", self.block_id, self.shard_id, global_idx, val);
            if val == 0 {
                break;
            }
            count = cvar.wait(count).unwrap();
        }
    }
}

// Copyright © Aptos Foundation

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use dashmap::DashSet;
use rayon::Scope;
use aptos_mvhashmap::MVHashMap;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::block_executor::partitioner::ShardId;
use aptos_types::executable::Executable;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::write_set::WriteOp;
use crate::errors::Error;
use crate::task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput};
use crate::txn_last_input_output::{TxnLastInputOutput, TxnOutput};

type LocalTxnIndex = TxnIndex;
type GlobalTxnIndex = TxnIndex;

#[derive(Clone, Debug)]
pub struct RemoteCommit<TO: TransactionOutput, TE: Debug> {
    pub global_txn_idx: GlobalTxnIndex,
    pub txn_output: Arc<TxnOutput<TO, TE>>,
}

pub enum ShardingMsg<TO: TransactionOutput, TE: Debug> {
    RemoteCommit(RemoteCommit<TO, TE>),
    Shutdown,
}

pub struct ShardingProvider<T: Transaction, TO: TransactionOutput, TE: Debug> {
    sharding_mode: bool,
    num_shards: usize,
    shard_id: usize,
    rx: Arc<Mutex<Receiver<ShardingMsg<TO, TE>>>>,
    senders: Vec<Mutex<Sender<ShardingMsg<TO, TE>>>>,

    /// Maps a local txn idx to the txn itself.
    pub(crate) txns: Vec<T>,
    /// Maps a global txn idx to its shard and in-shard txn idx.
    sharded_locations: Vec<(usize, LocalTxnIndex)>,
    /// Maps a local txn idx to its global idx.
    global_idxs: Vec<TxnIndex>,

    /// Maps a local txn idx to the number of remote txns it still waits for.
    missing_dep_counts: Vec<Arc<(Mutex<usize>, Condvar)>>,

    /// Maps a global txn idx to its followers.
    follower_sets: Vec<BTreeSet<(ShardId, GlobalTxnIndex)>>,
}

impl<TX, TO, TE> ShardingProvider<TX, TO, TE>
where
    TX: Transaction,
    TO: TransactionOutput<Txn = TX>,
    TE: Debug + Send + Clone,
{
    pub fn new_unsharded(txns: Vec<TX>) -> Self {
        let (tx, rx) = mpsc::channel();
        let num_txns = txns.len();
        Self {
            sharding_mode: false,
            num_shards: 1,
            shard_id: 0,
            rx: Arc::new(Mutex::new(rx)),
            senders: vec![],
            txns,
            sharded_locations: (0..num_txns).map(|idx|(0, idx as TxnIndex)).collect(),
            global_idxs: (0..(num_txns as TxnIndex)).collect(),
            missing_dep_counts: (0..num_txns).map(|idx|Arc::new((Mutex::new(0), Condvar::new()))).collect(),
            follower_sets: vec![],
        }
    }

    pub fn txn_by_global_idx(&self, idx: TxnIndex) -> &TX {
        let (shard_id, local_idx) = self.sharded_locations[idx as usize];
        assert_eq!(shard_id, self.shard_id);
        &self.txns[local_idx as usize]
    }

    pub fn global_idx_from_local(&self, local_idx: TxnIndex) -> TxnIndex {
        self.global_idxs[local_idx as usize]
    }

    pub fn nxt_global_idx(&self, idx: TxnIndex) -> Option<TxnIndex> {
        let (shard_id, local_idx) = self.sharded_locations[idx as usize];
        assert_eq!(shard_id, self.shard_id);
        self.global_idxs.get((local_idx + 1) as usize).copied()
    }

    pub fn num_local_txns(&self) -> usize {
        self.txns.len()
    }

    pub fn run_sharding_msg_loop<X: Executable + 'static>(&self, mv: &MVHashMap<TX::Key, TX::Value, X>) {
        if !self.sharding_mode { return; }

        loop {
            let msg = self.rx.lock().unwrap().recv().unwrap();
            match msg {
                ShardingMsg::RemoteCommit(msg) => {
                    let RemoteCommit { global_txn_idx, txn_output } = msg;
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
                    for &(_shard_id, cur_shard_follower) in cur_shard_followers {
                        let (_shard_id, follower_local_idx) = self.sharded_locations[cur_shard_follower as usize];
                        let (dep_counter_mutex, cvar) = &*self.missing_dep_counts[follower_local_idx as usize].clone();
                        let mut counter = dep_counter_mutex.lock().unwrap();
                        *counter -= 1;
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

    fn apply_updates_to_mv<X: Executable + 'static>(&self, versioned_cache: &MVHashMap<TX::Key, TX::Value, X>, global_txn_idx: GlobalTxnIndex, output: &TO) {
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
        let txn_output = txn_last_io.txn_output(txn_idx).unwrap();
        for shard_id in 0..self.num_shards {
            if shard_id == self.shard_id {
                continue;
            }
            if self.follower_sets[global_idx as usize].range((shard_id, 0)..(shard_id+1, 0)).next().is_some() {
                self.senders[shard_id].lock().unwrap().send(ShardingMsg::RemoteCommit(RemoteCommit { global_txn_idx: global_idx, txn_output: txn_output.clone() })).unwrap();
            }
        }
    }

    pub fn wait_for_remote_deps(&self, idx: LocalTxnIndex) {
        if !self.sharding_mode { return; }

        let (count_guard, cvar) = &*self.missing_dep_counts[idx as usize];
        let mut count = count_guard.lock().unwrap();
        while *count > 0 {
            count = cvar.wait(count).unwrap();
        }
    }
}

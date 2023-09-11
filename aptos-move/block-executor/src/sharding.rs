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

    /// Maps a remote txn to its write set that we need to wait for locally.
    pub remote_dependencies: HashMap<GlobalTxnIndex, HashSet<T::Key>>,

    /// Maps a local txn to every shard that contain at least 1 follower.
    pub following_shard_sets: Vec<Vec<usize>>,
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
            remote_dependencies: Default::default(),
            following_shard_sets: Default::default(),
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
        for &shard_id in &self.following_shard_sets[txn_idx as usize] {
            let msg = ShardingMsg::RemoteCommit(RemoteCommit {
                global_txn_idx: global_idx,
                txn_output: txn_output.clone(),
            });
            let sender = self.senders[shard_id].lock().unwrap();
            sender.send(msg).unwrap();
        }
    }
}

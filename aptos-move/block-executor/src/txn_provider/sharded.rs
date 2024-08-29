// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{
    scheduler::Scheduler,
    task::{ExecutionStatus, TransactionOutput},
    txn_last_input_output::TxnLastInputOutput,
    txn_provider::{BlockSTMPlugin, TxnIndexProvider},
};
use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_mvhashmap::{types::TxnIndex, MVHashMap};
use aptos_types::{executable::Executable, write_set::WriteOp, transaction::BlockExecutableTransaction as Transaction};
use dashmap::DashSet;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    sync::Arc,
};
use std::collections::BTreeMap;
use std::sync::{Condvar, Mutex};
use crossbeam::channel::Sender;
use aptos_types::transaction::signature_verified_transaction::SignatureVerifiedTransaction;
use move_core_types::value::MoveTypeLayout;
use serde::{Deserialize, Serialize};
use aptos_logger::info;
use crate::txn_commit_hook::OutputStreamHook;

#[derive(Deserialize, Serialize)]
pub enum CrossShardMessage<T: Transaction, TE: Debug> {
    Commit(CrossShardTxnResult<T, TE>),
    Shutdown,
}

#[derive(Deserialize, Serialize)]
pub struct CrossShardTxnResult<T: Transaction, TE: Debug> {
    pub global_txn_idx: TxnIndex,
    pub result: ExecutionStatus<ConcreteTxnOutput<T>, TE>,
}

pub trait CrossShardClientForV3<T: Transaction, TE: Debug>: Send + Sync {
    fn send(&self, shard_idx: usize, output: CrossShardMessage<T, TE>);
    fn recv(&self) -> CrossShardMessage<T, TE>;
}

/*pub enum BlockingTransactionStatus<T: Transaction> {
    Ready(Arc<SignatureVerifiedTransaction>),
    Waiting,
}*/

/*#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransactionIdxAndOutput {
    pub txn_idx: TxnIndex,
    pub txn_output: aptos_types::transaction::TransactionOutput,
}*/

pub enum BlockingTransactionStatus<T: Transaction> {
    Ready(Arc<T>),
    Waiting,
}

pub struct BlockingTransaction<T: Transaction> {
    pub txn: Mutex<BlockingTransactionStatus<T>>,
    pub cvar: Condvar,
}

impl <T: Transaction> BlockingTransaction<T> {
    pub fn new() -> Self {
        Self {
            txn: Mutex::new(BlockingTransactionStatus::Waiting),
            cvar: Condvar::new(),
        }
    }
}

pub enum ShardedTransaction<T: Transaction> {
    Txn(Arc<T>),
    BlockingTxn(BlockingTransaction<T>),
}

impl <T: Transaction> ShardedTransaction<T> {
    pub fn set_txn(&self, txn: T) {
        match self {
            ShardedTransaction::Txn(_) => {
                panic!("Trying to set a txn that is not a blocking txn");
            }
            ShardedTransaction::BlockingTxn(blocking_txn) => {
                let (lock, cvar) = (&blocking_txn.txn, &blocking_txn.cvar);
                let mut status = lock.lock().unwrap();
                match &*status {
                    BlockingTransactionStatus::Waiting => {
                        *status = BlockingTransactionStatus::Ready(Arc::new(txn));
                        cvar.notify_all();
                    },
                    BlockingTransactionStatus::Ready(_) => {
                        panic!("Trying to add a txn that is already present");
                    }
                }
            }
        }
    }
}

/// A BlockSTM plug-in that allows distributed execution with multiple BlockSTM instances.
pub struct ShardedTxnProvider<T: Transaction, TO: TransactionOutput, TE: Debug, L: OutputStreamHook<Output = TO>> {
    pub block_id: [u8; 32],
    pub num_shards: usize,
    pub shard_idx: usize,

    /// Maps a local txn idx to the txn itself.
    pub txns: Arc<Vec<ShardedTransaction<T>>>,
    /// Maps a global txn idx to its shard and in-shard txn idx.
    pub local_idxs_by_global: HashMap<TxnIndex, usize>,
    /// Maps a local txn idx to its global idx.
    pub global_idxs: Vec<TxnIndex>,

    /// Maps a remote txn to its write set that we need to wait for locally.
    pub remote_dependencies: HashMap<TxnIndex, HashSet<T::Key>>,

    /// Maps a local txn to every shard that contain at least 1 follower.
    pub follower_shard_sets: Vec<HashSet<usize>>,

    remote_committed_txns: DashSet<TxnIndex>,
    cross_shard_client: Arc<dyn CrossShardClientForV3<T, TE>>,
    output_stream_hook: Option<L>,
    phantom: PhantomData<TO>,
}

impl<T, TO, TE, L> ShardedTxnProvider<T, TO, TE, L>
where
    T: Transaction,
    TO: TransactionOutput<Txn = T>,
    TE: Debug + Send + Clone,
    L: OutputStreamHook<Output = TO>,
{
    pub fn new(
        block_id: [u8; 32],
        num_shards: usize,
        shard_idx: usize,
        cross_shard_client: Arc<dyn CrossShardClientForV3<T, TE>>,
        txns: Arc<Vec<ShardedTransaction<T>>>,
        global_idxs: Vec<TxnIndex>,
        local_idxs_by_global: HashMap<TxnIndex, usize>,
        remote_dependencies: HashMap<TxnIndex, HashSet<T::Key>>,
        follower_shard_sets: Vec<HashSet<usize>>,
        output_stream_hook: Option<L>,
    ) -> Self {
        //info!("Remote dependencies: {:?}", remote_dependencies);
        //info!("Follower shard sets: {:?}", follower_shard_sets);
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
            output_stream_hook,
            phantom: Default::default(),
        }
    }

    /*pub fn txn(&self, idx: TxnIndex) -> &T {
        self.get_txn(idx)
    }*/

    pub fn global_idx_from_local(&self, local_idx: TxnIndex) -> TxnIndex {
        self.global_idxs[local_idx as usize]
    }

    pub fn num_local_txns(&self) -> usize {
        self.txns.len()
    }

    /*pub fn set_txn(&self, idx: TxnIndex, txn: T) {
        let local_rank = self.local_idxs_by_global.get(&idx).copied().unwrap();
        match &self.txns[local_rank] {
            ShardedTransaction::Txn(_) => {
                panic!("Trying to set a txn that is not a blocking txn");
            }
            ShardedTransaction::BlockingTxn(blocking_txn) => {
                let (lock, cvar) = (&blocking_txn.txn, &blocking_txn.cvar);
                let mut status = lock.lock().unwrap();
                *status = BlockingTransactionStatus::Ready(Arc::new(txn));
                cvar.notify_all();
            }
        }
    }*/

    fn get_txn(&self, idx: TxnIndex) -> Arc<T> {
        let local_rank = self.local_idxs_by_global.get(&idx).copied().unwrap();
        match &self.txns[local_rank] {
            ShardedTransaction::Txn(txn) => txn.clone(),
            ShardedTransaction::BlockingTxn(blocking_txn) => {
                let (lock, cvar) = (&blocking_txn.txn, &blocking_txn.cvar);

                let mut status = lock.lock().unwrap();
                while let BlockingTransactionStatus::Waiting = *status {
                    status = cvar.wait(status).unwrap();
                }
                match &*status {
                    BlockingTransactionStatus::Ready(txn) => txn.clone(),
                    BlockingTransactionStatus::Waiting => unreachable!(),
                }
            }
        }
    }

    fn apply_updates_to_mv<X: Executable + 'static>(
        &self,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        global_txn_idx: TxnIndex,
        txn_output: ConcreteTxnOutput<T>,
    ) {
        let ConcreteTxnOutput {
            resource_write_set,
            module_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
        } = txn_output;
        // First, apply writes.
        /*(for (k, v) in resource_write_set
            .into_iter()
            .chain(aggregator_v1_write_set.into_iter())
        {
            versioned_cache.data().write(k, global_txn_idx, 0, v);
        }*/

        for (k, v, maybe_layout) in resource_write_set.into_iter().chain(
            aggregator_v1_write_set
                .into_iter()
                .map(|(state_key, write_op)| (state_key, Arc::new(write_op), None)),
        ) {
            versioned_cache
                .data()
                .write(k, global_txn_idx, 0, v, maybe_layout);
        }

        for (k, v) in module_write_set.into_iter() {
            versioned_cache.modules().write(k, global_txn_idx, v);
        }

        // Then, apply deltas.
        for (k, d) in aggregator_v1_delta_set.into_iter() {
            versioned_cache.data().add_delta(k, global_txn_idx, d);
        }
    }
}

impl<TX, TO, TE, L> TxnIndexProvider for ShardedTxnProvider<TX, TO, TE, L>
where
    TX: Transaction,
    TO: TransactionOutput<Txn = TX>,
    TE: Debug + Send + Clone,
    L: OutputStreamHook<Output = TO>,
{
    fn end_txn_idx(&self) -> TxnIndex {
        TxnIndex::MAX
    }

    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn first_txn(&self) -> TxnIndex {
        self.global_idxs
            .first()
            .copied()
            .unwrap_or(self.end_txn_idx())
    }

    fn next_txn(&self, idx: TxnIndex) -> TxnIndex {
        if idx == self.end_txn_idx() {
            self.end_txn_idx()
        } else {
            let local_rank = self.local_idxs_by_global.get(&idx).copied().unwrap();
            self.global_idxs
                .get(local_rank + 1)
                .copied()
                .unwrap_or(self.end_txn_idx())
        }
    }

    fn txns(&self) -> Vec<TxnIndex> {
        self.global_idxs.clone()
    }

    fn txns_and_deps(&self) -> Vec<TxnIndex> {
        let x = self.global_idxs.iter();
        let y = self.remote_dependencies.keys();
        x.chain(y)
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn local_index(&self, idx: TxnIndex) -> usize {
        match self.local_idxs_by_global.get(&idx).copied() {
            Some(local_idx) => local_idx,
            None => {
                panic!(
                    "Local index not found for global index {:?} in shard {:?}",
                    idx, self.shard_idx
                );
            }
        }
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

impl<TX, TO, TE, L> BlockSTMPlugin<TX, TO, TE> for ShardedTxnProvider<TX, TO, TE, L>
where
    TX: Transaction,
    TO: TransactionOutput<Txn = TX>,
    TE: Debug + Send + Clone,
    L: OutputStreamHook<Output = TO>,
{
    fn remote_dependencies(&self) -> Vec<(TxnIndex, TX::Key)> {
        self.remote_dependencies
            .iter()
            .flat_map(|(txn_idx, keys)| keys.iter().map(|key| (*txn_idx, key.clone())))
            .collect()
    }

    fn txn(&self, idx: TxnIndex) -> Arc<TX> {
        self.get_txn(idx)
    }

    fn run_sharding_msg_loop<X: Executable + 'static>(
        &self,
        mv: &MVHashMap<TX::Key, TX::Tag, TX::Value, X, TX::Identifier>,
        scheduler: &Scheduler<Self>,
    ) {
        if self.remote_dependencies.is_empty() {
            return;
        }
        while let CrossShardMessage::Commit(CrossShardTxnResult {
            global_txn_idx,
            result,
        }) = self.cross_shard_client.recv()
        {
            //info!("Received cross shard commit message for txn_idx {}", global_txn_idx);
            match result {
                ExecutionStatus::Success(output) => {
                    self.apply_updates_to_mv(mv, global_txn_idx, output);
                },
                ExecutionStatus::SkipRest(output) => {
                    self.apply_updates_to_mv(mv, global_txn_idx, output);
                },
                ExecutionStatus::Abort(_) => {
                    //sharding todo: what to do here?
                },
                ExecutionStatus::SpeculativeExecutionAbortError(_) => {
                    //sharding todo: what to do here?
                },
                ExecutionStatus::DelayedFieldsCodeInvariantError(_) => {
                    //sharding todo: what to do here?
                },
            }
            self.remote_committed_txns.insert(global_txn_idx);
            scheduler.fast_resume_dependents(global_txn_idx);
        }
    }

    fn shutdown_receiver(&self) {
        self.cross_shard_client
            .send(self.shard_idx, CrossShardMessage::Shutdown);
    }

    fn on_local_commit(
        &self,
        txn_idx: TxnIndex,
        last_input_output: &TxnLastInputOutput<TX, TO, TE>,
        _delta_writes: &[(TX::Key, WriteOp)],
    ) {
        let txn_output = last_input_output.txn_output(txn_idx).unwrap();
        let concrete_status = match txn_output.as_ref() {
            ExecutionStatus::Success(obj) => {
                /*if let Some(output_stream_hook) = &self.output_stream_hook {
                    output_stream_hook.stream_output(txn_idx, obj);
                }*/
                ExecutionStatus::Success(ConcreteTxnOutput::new(obj))
            },
            ExecutionStatus::SkipRest(obj) => {
                /*if let Some(output_stream_hook) = &self.output_stream_hook {
                    output_stream_hook.stream_output(txn_idx, obj);
                }*/
                ExecutionStatus::SkipRest(ConcreteTxnOutput::new(obj))
            },
            ExecutionStatus::Abort(obj) => ExecutionStatus::Abort(obj.clone()),
            ExecutionStatus::SpeculativeExecutionAbortError(obj) => {
                ExecutionStatus::SpeculativeExecutionAbortError(obj.clone())
            },
            ExecutionStatus::DelayedFieldsCodeInvariantError(obj) => {
                ExecutionStatus::DelayedFieldsCodeInvariantError(obj.clone())
            },
        };

        let txn_local_index = self.local_index(txn_idx);
        for &shard_id in &self.follower_shard_sets[txn_local_index] {
            info!("Sending cross shard commit message to shard {} for txn_idx {}", shard_id, txn_idx);
            self.cross_shard_client.send(
                shard_id,
                CrossShardMessage::Commit(CrossShardTxnResult {
                    global_txn_idx: txn_idx,
                    result: concrete_status.clone(),
                }),
            );
        }
    }

    fn stream_output(
        &self,
        txn_idx: TxnIndex,
        last_input_output: &TxnLastInputOutput<TX, TO, TE>,
    ) {
        if self.output_stream_hook.is_none() {
            return;
        }
        //info!("Streaming output for txn_idx {}", txn_idx);
        let txn_output = last_input_output.txn_output(txn_idx).unwrap();
        match txn_output.as_ref() {
            ExecutionStatus::Success(obj) => {
               // info!("ExecutionStatus::Success for txn_idx {}", txn_idx);
                if let Some(output_stream_hook) = &self.output_stream_hook {
                    output_stream_hook.stream_output(txn_idx, obj);
                }
            },
            ExecutionStatus::SkipRest(obj) => {
                if let Some(output_stream_hook) = &self.output_stream_hook {
                    output_stream_hook.stream_output(txn_idx, obj);
                }
            },
            ExecutionStatus::Abort(obj) => {},
            ExecutionStatus::SpeculativeExecutionAbortError(obj) => {},
            ExecutionStatus::DelayedFieldsCodeInvariantError(obj) => {},
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ConcreteTxnOutput<T: Transaction> {
    pub resource_write_set: Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)>,
    pub module_write_set: BTreeMap<T::Key, T::Value>,
    pub aggregator_v1_write_set: BTreeMap<T::Key, T::Value>,
    pub aggregator_v1_delta_set: Vec<(T::Key, DeltaOp)>,
}

impl<T: Transaction> ConcreteTxnOutput<T> {
    pub fn new<TO: TransactionOutput<Txn = T>>(txn_output: &TO) -> Self {
        Self {
            resource_write_set: txn_output.resource_write_set(),
            module_write_set: txn_output.module_write_set(),
            aggregator_v1_write_set: txn_output.aggregator_v1_write_set(),
            aggregator_v1_delta_set: txn_output.aggregator_v1_delta_set(),
        }
    }
}

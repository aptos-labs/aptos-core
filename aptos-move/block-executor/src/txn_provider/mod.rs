// Copyright © Aptos Foundation

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Debug;
use std::slice::Iter;
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
use crate::scheduler::Scheduler;
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

pub trait TxnProviderTrait1 {
    fn end_txn_idx(&self) -> TxnIndex;
    fn num_txns(&self) -> usize;
    fn first_txn(&self) -> TxnIndex;
    fn next_txn(&self, idx: TxnIndex) -> TxnIndex;
    fn txns(&self) -> Vec<TxnIndex>;
    fn txns_and_deps(&self) -> Vec<TxnIndex>;
    fn local_rank(&self, idx: TxnIndex) -> usize;
    fn is_local(&self, idx: TxnIndex) -> bool;
    fn txn_output_has_arrived(&self, txn_idx: TxnIndex) -> bool;
    fn block_idx(&self) -> &[u8]; //debug only
    fn shard_idx(&self) -> usize;
}

pub trait TxnProviderTrait2<T: Transaction, TO, TE>
    where
        T: Transaction,
        TO: TransactionOutput<Txn = T>,
        TE: Debug + Send + Clone,
{
    fn remote_dependencies(&self) -> Vec<(TxnIndex, T::Key)>;
    fn run_sharding_msg_loop<TAG, X: Executable + 'static>(&self, mv_cache: &MVHashMap<T::Key, TAG, T::Value, X>, scheduler: &Scheduler<Self>);
    fn shutdown_receiver(&self);
    fn txn(&self, idx: TxnIndex) -> &T;
    fn on_local_commit(&self, txn_idx: TxnIndex, txn_output: Arc<TxnOutput<TO, TE>>);
    fn commit_strategy(&self) -> u8;
}

pub mod default;
pub mod sharded;

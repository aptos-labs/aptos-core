// Copyright © Aptos Foundation

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::slice::Iter;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use dashmap::DashSet;
use rayon::Scope;
use serde::Serialize;
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

/// The transaction index operations that are implemented differently between unsharded execution and sharded execution.
pub trait TxnIndexProvider {
    /// Get the special `TxnIndex` used in BlockSTM that indicates the end of the local txn list.
    fn end_txn_idx(&self) -> TxnIndex;

    /// Get number of local txns.
    fn num_txns(&self) -> usize;

    /// Get the 1st local txn.
    fn first_txn(&self) -> TxnIndex;

    /// Given the global index of a local txn, return the global index of the one right after in the local txn list.
    fn next_txn(&self, idx: TxnIndex) -> TxnIndex;

    /// Get the global indices of all local txns.
    fn txns(&self) -> Vec<TxnIndex>;

    /// Get the global indices of all local txns + their remote dependencies.
    fn txns_and_deps(&self) -> Vec<TxnIndex>;

    /// Given the global index of a local txn, return its index in the local sub-sequence.
    fn local_index(&self, idx: TxnIndex) -> usize;

    /// Given a global txn index, return whether the txn is assigned to the current shard.
    fn is_local(&self, idx: TxnIndex) -> bool;

    /// Given the global index of a remote txn, return whether its output has arrived.
    fn txn_output_has_arrived(&self, txn_idx: TxnIndex) -> bool;

    /// Get the block ID.
    fn block_idx(&self) -> &[u8]; //debug only

    /// Get the index of the current shard.
    fn shard_idx(&self) -> usize;
}

/// Some other places where unsharded execution and sharded execution work differently.
pub trait BlockSTMPlugin<T: Transaction, TO, TE>
    where
        T: Transaction,
        TO: TransactionOutput<Txn = T>,
        TE: Debug + Send + Clone,
{
    /// Get all the remote dependency set of all local txns.
    fn remote_dependencies(&self) -> Vec<(TxnIndex, T::Key)>;

    /// Get a reference of the txn object by its global index.
    fn txn(&self, idx: TxnIndex) -> &T;

    /// Run a loop to receive remote txn output and unblock local txns.
    fn run_sharding_msg_loop<X: Executable + 'static>(&self, mv_cache: &MVHashMap<T::Key, T::Tag, T::Value, X>, scheduler: &Scheduler<Self>);

    /// Stop the loop above.
    fn shutdown_receiver(&self);

    /// Some extra processing once a local txn is committed.
    fn on_local_commit(&self, txn_idx: TxnIndex, last_input_output: &TxnLastInputOutput<T, TO, TE>, delta_writes: &Vec<(T::Key, WriteOp)>);

    /// Return whether a dedicated committing thread should be used in BlockSTM.
    fn use_dedicated_committing_thread(&self) -> bool;
}

pub mod default;
pub mod sharded;

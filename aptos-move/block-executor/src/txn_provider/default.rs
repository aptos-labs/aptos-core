// Copyright © Aptos Foundation
use std::fmt::Debug;
use aptos_mvhashmap::MVHashMap;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::executable::Executable;
use aptos_types::write_set::WriteOp;
use move_core_types::account_address::AccountAddress;
use crate::scheduler::Scheduler;
use crate::task::{Transaction, TransactionOutput};
use crate::txn_last_input_output::TxnLastInputOutput;
use crate::txn_provider::{TxnIndexProvider, BlockSTMPlugin};

/// Some logic of vanilla BlockSTM.
pub struct DefaultTxnProvider<T> {
    block_id: [u8; 32],
    txns: Vec<T>,
}

impl<T> DefaultTxnProvider<T> {
    pub fn new(txns: Vec<T>) -> Self {
        Self {
            block_id: AccountAddress::random().into_bytes(),
            txns
        }
    }
}

impl<T> TxnIndexProvider for DefaultTxnProvider<T> {
    fn end_txn_idx(&self) -> TxnIndex {
        self.txns.len() as TxnIndex
    }

    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn first_txn(&self) -> TxnIndex {
        if self.num_txns() == 0 { self.end_txn_idx() } else { 0 }
    }

    fn next_txn(&self, idx: TxnIndex) -> TxnIndex {
        if idx == self.end_txn_idx() { idx } else { idx + 1 }
    }

    fn txns(&self) -> Vec<TxnIndex> {
        (0..self.num_txns() as TxnIndex).collect()
    }

    fn txns_and_deps(&self) -> Vec<TxnIndex> {
        self.txns()
    }

    fn local_index(&self, idx: TxnIndex) -> usize {
        idx as usize
    }

    fn is_local(&self, _idx: TxnIndex) -> bool {
        true
    }

    fn txn_output_has_arrived(&self, _txn_idx: TxnIndex) -> bool {
        unreachable!()
    }

    fn block_idx(&self) -> &[u8] {
        self.block_id.as_slice()
    }

    fn shard_idx(&self) -> usize {
        0
    }
}

impl<T, TO, TE> BlockSTMPlugin<T, TO, TE> for DefaultTxnProvider<T>
    where
        T: Transaction,
        TO: TransactionOutput<Txn = T>,
        TE: Debug + Send + Clone,
{
    fn remote_dependencies(&self) -> Vec<(TxnIndex, T::Key)> {
        vec![]
    }

    fn run_sharding_msg_loop<X: Executable + 'static>(&self, _mv_cache: &MVHashMap<T::Key, T::Tag, T::Value, X>, _scheduler: &Scheduler<Self>) {
        // Nothing to do.
    }

    fn shutdown_receiver(&self) {
        // Nothing to do.
    }

    fn txn(&self, idx: TxnIndex) -> &T {
        &self.txns[idx as usize]
    }

    fn on_local_commit(&self, _txn_idx: TxnIndex, _last_input_output: &TxnLastInputOutput<T, TO, TE>, _delta_writes: &[(T::Key, WriteOp)]) {
        // Nothing to do.
    }

    fn use_dedicated_committing_thread(&self) -> bool {
        false
    }
}

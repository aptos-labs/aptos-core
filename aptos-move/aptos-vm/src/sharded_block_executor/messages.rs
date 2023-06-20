// Copyright © Aptos Foundation

use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};

#[derive(Clone, Debug)]
pub enum CrossShardMsg {
    RemoteTxnCommitMsg(RemoteTxnCommit),
    StopMsg,
}

#[derive(Clone, Debug)]
pub struct RemoteTxnCommit {
    txn_index: TxnIndex,
    txn_writes: Vec<(StateKey, WriteOp)>,
}

impl RemoteTxnCommit {
    pub fn new(txn_index: TxnIndex, txn_writes: Vec<(StateKey, WriteOp)>) -> Self {
        Self {
            txn_index,
            txn_writes,
        }
    }

    pub fn take(self) -> (TxnIndex, Vec<(StateKey, WriteOp)>) {
        (self.txn_index, self.txn_writes)
    }
}

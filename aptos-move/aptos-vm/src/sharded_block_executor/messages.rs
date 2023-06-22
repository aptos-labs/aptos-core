// Copyright © Aptos Foundation

use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};

#[derive(Clone, Debug)]
pub enum CrossShardMsg {
    RemoteTxnWriteMsg(RemoteTxnWrite),
    StopMsg,
}

#[derive(Clone, Debug)]
pub struct RemoteTxnWrite {
    txn_index: TxnIndex,
    state_key: StateKey,
    write_op: WriteOp,
}

impl RemoteTxnWrite {
    pub fn new(txn_index: TxnIndex, state_key: StateKey, write_op: WriteOp) -> Self {
        Self {
            txn_index,
            state_key,
            write_op,
        }
    }

    pub fn take(self) -> (TxnIndex, StateKey, WriteOp) {
        (self.txn_index, self.state_key, self.write_op)
    }
}

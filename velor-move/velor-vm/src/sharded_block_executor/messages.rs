// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_types::{state_store::state_key::StateKey, write_set::WriteOp};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CrossShardMsg {
    RemoteTxnWriteMsg(RemoteTxnWrite),
    StopMsg,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoteTxnWrite {
    state_key: StateKey,
    // The write op is None if the transaction is aborted.
    write_op: Option<WriteOp>,
}

impl RemoteTxnWrite {
    pub fn new(state_key: StateKey, write_op: Option<WriteOp>) -> Self {
        Self {
            state_key,
            write_op,
        }
    }

    pub fn take(self) -> (StateKey, Option<WriteOp>) {
        (self.state_key, self.write_op)
    }
}

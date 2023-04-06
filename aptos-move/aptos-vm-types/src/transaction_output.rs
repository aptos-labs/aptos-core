// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0


use aptos_types::contract_event::ContractEvent;
use aptos_types::transaction::{TransactionOutput, TransactionStatus};
use crate::change_set::{AptosChangeSet, ChangeSet};
use crate::delta::DeltaOp;
use crate::remote_cache::StateViewWithRemoteCache;
use crate::write::{AptosWrite, Op};

/// Output of a transaction at VM level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VMTransactionOutput {
    /// The list of writes this transaction intends to do.
    writes: ChangeSet<Op<AptosWrite>>,

    /// The list of deltas this transaction intends to apply
    deltas: ChangeSet<DeltaOp>,

    /// The list of events emitted during this transaction.
    events: Vec<ContractEvent>,

    /// The amount of gas used during execution.
    gas_used: u64,

    /// The execution status.
    status: TransactionStatus,
}

impl VMTransactionOutput {
    pub fn new(
        writes: ChangeSet<Op<AptosWrite>>,
        deltas: ChangeSet<DeltaOp>,
        events: Vec<ContractEvent>,
        gas_used: u64,
        status: TransactionStatus,
    ) -> Self {
        Self {
            writes,
            deltas,
            events,
            gas_used,
            status,
        }
    }

    pub fn into(self) -> (ChangeSet<Op<AptosWrite>>, ChangeSet<DeltaOp>, Vec<ContractEvent>) {
        (self.writes, self.deltas, self.events)
    }

    pub fn unpack(self) -> (ChangeSet<Op<AptosWrite>>, ChangeSet<DeltaOp>, Vec<ContractEvent>, u64, TransactionStatus) {
        (self.writes, self.deltas, self.events, self.gas_used, self.status)
    }

    pub fn writes(&self) -> &ChangeSet<Op<AptosWrite>> {
        &self.writes
    }

    pub fn deltas(&self) -> &ChangeSet<DeltaOp> {
        &self.deltas
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }

    pub fn status(&self) -> &TransactionStatus {
        &self.status
    }
}

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::{DeltaChangeSet, WriteChangeSet};
use aptos_types::{contract_event::ContractEvent, transaction::TransactionStatus};

/// The output of executing a transaction.
#[derive(Debug)]
pub struct ExecutionOutput {
    /// The list of deltas this transaction intends to apply.
    delta_changes: DeltaChangeSet,

    /// The list of writes this transaction intends to do.
    write_changes: WriteChangeSet,

    /// The list of events emitted during this transaction.
    events: Vec<ContractEvent>,

    /// The amount of gas used during execution.
    gas_used: u64,

    /// The execution status.
    status: TransactionStatus,
}

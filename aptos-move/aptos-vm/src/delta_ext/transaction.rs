// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::delta_ext::DeltaChangeSet;
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{ChangeSet, TransactionOutput, TransactionStatus},
    write_set::WriteSet,
};

/// Extension of `ChangeSet` that also holds deltas.
pub struct ChangeSetExt {
    delta_change_set: DeltaChangeSet,
    change_set: ChangeSet,
}

impl ChangeSetExt {
    pub fn new(delta_change_set: DeltaChangeSet, change_set: ChangeSet) -> Self {
        ChangeSetExt {
            delta_change_set,
            change_set,
        }
    }

    pub fn into_inner(self) -> (DeltaChangeSet, ChangeSet) {
        (self.delta_change_set, self.change_set)
    }
}

/// Extension of `TransactionOutput` that also holds `DeltaChangeSet`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionOutputExt {
    delta_change_set: DeltaChangeSet,
    output: TransactionOutput,
}

impl TransactionOutputExt {
    pub fn new(delta_change_set: DeltaChangeSet, output: TransactionOutput) -> Self {
        TransactionOutputExt {
            delta_change_set,
            output,
        }
    }

    pub fn delta_change_set(&self) -> &DeltaChangeSet {
        &self.delta_change_set
    }

    pub fn write_set(&self) -> &WriteSet {
        self.output.write_set()
    }

    pub fn events(&self) -> &[ContractEvent] {
        self.output.events()
    }

    pub fn gas_used(&self) -> u64 {
        self.output.gas_used()
    }

    pub fn status(&self) -> &TransactionStatus {
        self.output.status()
    }

    pub fn into(self) -> (DeltaChangeSet, TransactionOutput) {
        (self.delta_change_set, self.output)
    }
}

impl From<TransactionOutput> for TransactionOutputExt {
    fn from(output: TransactionOutput) -> Self {
        TransactionOutputExt {
            delta_change_set: DeltaChangeSet::empty(),
            output,
        }
    }
}

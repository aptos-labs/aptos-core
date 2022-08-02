// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::delta_ext::DeltaChangeSet;
use aptos_state_view::StateView;
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{ChangeSet, TransactionOutput, TransactionStatus},
    vm_status::VMStatus,
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

    /// Similar to `into()` but tries to apply delta changes as well.
    pub fn into_transaction_output_with_status(
        self,
        state_view: &impl StateView,
    ) -> (VMStatus, TransactionOutput) {
        let (delta_change_set, txn_output) = self.into();

        // No deltas - return immediately.
        if delta_change_set.is_empty() {
            return (VMStatus::Executed, txn_output);
        }

        match delta_change_set.try_into_write_set_mut(state_view) {
            Err(_) => {
                // TODO: at this point we know that delta application failed
                // (and it should have occurred in user transaction in general).
                // We need to rerun the epilogue and charge gas. Since we
                // support only a limited set of features for the aggregator
                // anyway, for now - panic.
                panic!("something terrible happened when applying aggregator deltas")
            }
            Ok(mut materialized_deltas) => {
                let (write_set, events, gas_used, status) = txn_output.unpack();
                // We expect to have only a few delta changes, so add them to
                // the write set of the transaction.
                let mut write_set_mut = write_set.into_mut();
                write_set_mut.append(&mut materialized_deltas);

                let output = TransactionOutput::new(
                    write_set_mut.freeze().unwrap(),
                    events,
                    gas_used,
                    status,
                );

                (VMStatus::Executed, output)
            }
        }
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

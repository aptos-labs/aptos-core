// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::delta_change_set::DeltaChangeSet;
use aptos_state_view::StateView;
use aptos_types::{
    transaction::{ChangeSet, TransactionOutput},
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

    pub fn delta_change_set(&self) -> &DeltaChangeSet {
        &self.delta_change_set
    }

    pub fn write_set(&self) -> &WriteSet {
        self.change_set.write_set()
    }

    pub fn into_inner(self) -> (DeltaChangeSet, ChangeSet) {
        (self.delta_change_set, self.change_set)
    }

    pub fn squash(self, other: Self) -> anyhow::Result<Self> {
        Ok(Self {
            delta_change_set: self.delta_change_set.merge_with(other.delta_change_set)?,
            change_set: self.change_set.squash(other.change_set)?,
        })
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

    pub fn txn_output(&self) -> &TransactionOutput {
        &self.output
    }

    pub fn into(self) -> (DeltaChangeSet, TransactionOutput) {
        (self.delta_change_set, self.output)
    }

    /// Similar to `into()` but tries to apply delta changes as well.
    /// TODO: ideally, we may want to expose this function to VM instead. Since
    /// we do not care about rerunning the epilogue - it sufficies to have it
    /// here for now.
    pub fn into_transaction_output(
        self,
        state_view: &impl StateView,
    ) -> Result<TransactionOutput, VMStatus> {
        let (delta_change_set, txn_output) = self.into();

        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if txn_output.status().is_discarded() || delta_change_set.is_empty() {
            return Ok(txn_output);
        }

        match delta_change_set.try_into_write_set_mut(state_view) {
            Err(_) => {
                // TODO: at this point we know that delta application failed
                // (and it should have occurred in user transaction in general).
                // We need to rerun the epilogue and charge gas. Currently, the use
                // case of an aggregator is for gas fees (which are computed in
                // the epilogue), and therefore this should never happen.
                // Also, it is worth mentioning that current VM error handling is
                // rather ugly and has a lot of legacy code. This makes proper error
                // handling quite challenging.
                panic!("something terrible happened when applying aggregator deltas");
            }
            Ok(materialized_deltas) => {
                let (write_set, events, gas_used, status) = txn_output.unpack();
                // We expect to have only a few delta changes, so add them to
                // the write set of the transaction.
                let write_set_mut = write_set.into_mut().squash(materialized_deltas).unwrap();

                let output = TransactionOutput::new(
                    write_set_mut.freeze().unwrap(),
                    events,
                    gas_used,
                    status,
                );

                Ok(output)
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

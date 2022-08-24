// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::delta_change_set::{deserialize, DeltaChangeSet};
use anyhow::bail;
use aptos_state_view::StateView;
use aptos_types::write_set::TransactionWrite;
use aptos_types::{
    transaction::{ChangeSet, TransactionOutput},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use std::collections::btree_map;

/// Helpful trait for e.g. extracting u128 value out of TransactionWrite that we know is
/// for aggregator (i.e. if we have seen a DeltaOp for the same access path).
pub struct AggregatorValue(u128);

impl AggregatorValue {
    /// Returns None if the write doesn't contain a value (i.e deletion), and panics if
    /// the value raw bytes can't be deserialized into an u128.
    pub fn from_write(write: &dyn TransactionWrite) -> Option<Self> {
        let v = write.extract_raw_bytes();
        v.map(|bytes| Self(deserialize(&bytes)))
    }

    pub fn into(self) -> u128 {
        self.0
    }
}

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

    pub fn squash_delta_change_set(self, other: DeltaChangeSet) -> anyhow::Result<Self> {
        use btree_map::Entry::*;
        use WriteOp::*;

        let (mut delta, change_set) = self.into_inner();
        let (write_set, events) = change_set.into_inner();
        let mut write_set = write_set.into_mut();

        let delta_ops = delta.as_inner_mut();
        let write_ops = write_set.as_inner_mut();

        for (key, op) in other.into_iter() {
            if let Some(r) = write_ops.get_mut(&key) {
                match r {
                    Creation(data) => {
                        let val: u128 = bcs::from_bytes(data)?;
                        *r = Creation(bcs::to_bytes(&op.apply_to(val)?)?);
                    }
                    Modification(data) => {
                        let val: u128 = bcs::from_bytes(data)?;
                        *r = Modification(bcs::to_bytes(&op.apply_to(val)?)?);
                    }
                    Deletion => {
                        bail!("Failed to apply Aggregator delta -- value already deleted");
                    }
                }
            } else {
                match delta_ops.entry(key) {
                    Occupied(entry) => {
                        entry.into_mut().merge_with(op)?;
                    }
                    Vacant(entry) => {
                        entry.insert(op);
                    }
                }
            }
        }

        Ok(Self {
            delta_change_set: delta,
            change_set: ChangeSet::new(write_set.freeze()?, events),
        })
    }

    pub fn squash_change_set(self, other: ChangeSet) -> anyhow::Result<Self> {
        use btree_map::Entry::*;
        use WriteOp::*;

        let (mut delta, change_set) = self.into_inner();
        let (write_set, mut events) = change_set.into_inner();
        let mut write_set = write_set.into_mut();
        let write_ops = write_set.as_inner_mut();

        let (other_write_set, other_events) = other.into_inner();

        for (key, op) in other_write_set.into_iter() {
            match write_ops.entry(key) {
                Occupied(mut entry) => {
                    let r = entry.get_mut();
                    match (&r, op) {
                        (Modification(_) | Creation(_), Creation(_))
                        | (Deletion, Deletion | Modification(_)) => {
                            bail!("The given change sets cannot be squashed")
                        }
                        (Modification(_), Modification(data)) => *r = Modification(data),
                        (Creation(_), Modification(data)) => *r = Creation(data),
                        (Modification(_), Deletion) => *r = Deletion,
                        (Deletion, Creation(data)) => *r = Modification(data),
                        (Creation(_), Deletion) => {
                            entry.remove();
                        }
                    }
                }
                Vacant(entry) => {
                    delta.remove(entry.key());
                    entry.insert(op);
                }
            }
        }

        events.extend(other_events);

        Ok(Self {
            delta_change_set: delta,
            change_set: ChangeSet::new(write_set.freeze()?, events),
        })
    }

    pub fn squash(self, other: Self) -> anyhow::Result<Self> {
        let (delta_change_set, change_set) = other.into_inner();
        self.squash_change_set(change_set)?
            .squash_delta_change_set(delta_change_set)
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
    pub fn into_transaction_output(self, state_view: &impl StateView) -> TransactionOutput {
        let (delta_change_set, txn_output) = self.into();

        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if txn_output.status().is_discarded() || delta_change_set.is_empty() {
            return txn_output;
        }

        // TODO: at this point we know that delta application failed
        // (and it should have occurred in user transaction in general).
        // We need to rerun the epilogue and charge gas. Currently, the use
        // case of an aggregator is for gas fees (which are computed in
        // the epilogue), and therefore this should never happen.
        // Also, it is worth mentioning that current VM error handling is
        // rather ugly and has a lot of legacy code. This makes proper error
        // handling quite challenging.
        delta_change_set
            .try_into_write_set_mut(state_view)
            .map(|materialized_deltas| Self::merge_delta_writes(txn_output, materialized_deltas))
            .expect("Failed to apply aggregator delta outputs")
    }

    pub fn output_with_delta_writes(self, delta_writes: WriteSetMut) -> TransactionOutput {
        let (delta_change_set, txn_output) = self.into();

        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if txn_output.status().is_discarded() || delta_change_set.is_empty() {
            return txn_output;
        }

        // We should have a delta write for every delta in the output.
        assert_eq!(delta_change_set.len(), delta_writes.len());

        Self::merge_delta_writes(txn_output, delta_writes)
    }

    fn merge_delta_writes(
        output: TransactionOutput,
        delta_writes: WriteSetMut,
    ) -> TransactionOutput {
        let (write_set, events, gas_used, status) = output.unpack();
        // We expect to have only a few delta changes, so add them to
        // the write set of the transaction.
        let write_set_mut = write_set.into_mut();
        // TODO: Is it okay to panic here?
        let write_set_mut = write_set_mut.squash(delta_writes).unwrap();

        TransactionOutput::new(write_set_mut.freeze().unwrap(), events, gas_used, status)
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

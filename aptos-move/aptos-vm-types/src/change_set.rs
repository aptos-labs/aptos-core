// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::check_change_set::CheckChangeSet;
use aptos_aggregator::delta_change_set::{deserialize, serialize, DeltaOp};
use aptos_state_view::StateView;
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::{StateKey, StateKeyInner},
    transaction::ChangeSet as StorageChangeSet,
    write_set::{WriteOp, WriteSetMut},
};
use move_binary_format::errors::Location;
use move_core_types::vm_status::{err_msg, StatusCode, VMStatus};
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};

/// A change set produced by the VM.
///
/// **WARNING**: Just like VMOutput, this type should only be used inside the
/// VM. For storage backends, use `ChangeSet`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VMChangeSet {
    resource_write_set: HashMap<StateKey, WriteOp>,
    module_write_set: HashMap<StateKey, WriteOp>,
    aggregator_write_set: HashMap<StateKey, WriteOp>,
    aggregator_delta_set: HashMap<StateKey, DeltaOp>,
    events: Vec<ContractEvent>,
}

macro_rules! squash_writes_pair {
    ($write_entry:ident, $additional_write:ident) => {
        // Squashing creation and deletion is a no-op. In that case, we
        // have to remove the old write op from the write set.
        let noop = !WriteOp::squash($write_entry.get_mut(), $additional_write).map_err(|e| {
            VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg(format!("Error while squashing two write ops: {}.", e)),
            )
        })?;
        if noop {
            $write_entry.remove();
        }
    };
}

impl VMChangeSet {
    pub fn empty() -> Self {
        Self {
            resource_write_set: HashMap::new(),
            module_write_set: HashMap::new(),
            aggregator_write_set: HashMap::new(),
            aggregator_delta_set: HashMap::new(),
            events: vec![],
        }
    }

    pub fn new(
        resource_write_set: HashMap<StateKey, WriteOp>,
        module_write_set: HashMap<StateKey, WriteOp>,
        aggregator_write_set: HashMap<StateKey, WriteOp>,
        aggregator_delta_set: HashMap<StateKey, DeltaOp>,
        events: Vec<ContractEvent>,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let change_set = Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            aggregator_delta_set,
            events,
        };

        // Returns an error if structure of the change set is not valid,
        // e.g. the size in bytes is too large.
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    /// Builds a new change set from the storage representation.
    ///
    /// **WARNING**: Has complexity O(#write_ops) because we need to iterate
    /// over blobs and split them into resources or modules. Only used to
    /// support transactions with write-set payload.
    pub fn try_from_storage_change_set(
        change_set: StorageChangeSet,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let (write_set, events) = change_set.into_inner();

        // There should be no aggregator writes if we have a change set from
        // storage.
        let mut resource_write_set = HashMap::new();
        let mut module_write_set = HashMap::new();

        for (state_key, write_op) in write_set {
            if matches!(state_key.inner(), StateKeyInner::AccessPath(ap) if ap.is_code()) {
                module_write_set.insert(state_key, write_op);
            } else {
                // TODO(aggregator) While everything else must be a resource, first
                // version of aggregators is implemented as a table item. Revisit when
                // we split MVHashMap into data and aggregators.
                resource_write_set.insert(state_key, write_op);
            }
        }

        let change_set = Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set: HashMap::new(),
            aggregator_delta_set: HashMap::new(),
            events,
        };
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    pub(crate) fn into_storage_change_set_unchecked(self) -> StorageChangeSet {
        let Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            aggregator_delta_set: _,
            events,
        } = self;

        let mut write_set_mut = WriteSetMut::default();
        write_set_mut.extend(resource_write_set);
        write_set_mut.extend(module_write_set);
        write_set_mut.extend(aggregator_write_set);

        let write_set = write_set_mut
            .freeze()
            .expect("Freezing a WriteSet does not fail.");
        StorageChangeSet::new(write_set, events)
    }

    /// Converts VM-native change set into its storage representation with fully
    /// serialized changes.
    /// If deltas are not materialized the conversion fails.
    pub fn try_into_storage_change_set(self) -> anyhow::Result<StorageChangeSet, VMStatus> {
        if !self.aggregator_delta_set.is_empty() {
            return Err(VMStatus::error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg(
                    "Cannot convert from VMChangeSet with non-materialized deltas to ChangeSet.",
                ),
            ));
        }
        Ok(self.into_storage_change_set_unchecked())
    }

    pub fn write_set_iter(&self) -> impl Iterator<Item = (&StateKey, &WriteOp)> {
        self.resource_write_set
            .iter()
            .chain(self.module_write_set.iter())
            .chain(self.aggregator_write_set.iter())
    }

    pub fn write_set_iter_mut(&mut self) -> impl Iterator<Item = (&StateKey, &mut WriteOp)> {
        self.resource_write_set
            .iter_mut()
            .chain(self.module_write_set.iter_mut())
            .chain(self.aggregator_write_set.iter_mut())
    }

    pub fn resource_write_set(&self) -> &HashMap<StateKey, WriteOp> {
        &self.resource_write_set
    }

    pub fn module_write_set(&self) -> &HashMap<StateKey, WriteOp> {
        &self.module_write_set
    }

    // Called by `try_into_transaction_output_with_materialized_writes` only.
    pub(crate) fn extend_aggregator_write_set(
        &mut self,
        additional_aggregator_writes: impl Iterator<Item = (StateKey, WriteOp)>,
    ) {
        self.aggregator_write_set
            .extend(additional_aggregator_writes)
    }

    pub fn aggregator_v1_write_set(&self) -> &HashMap<StateKey, WriteOp> {
        &self.aggregator_write_set
    }

    pub fn aggregator_v1_delta_set(&self) -> &HashMap<StateKey, DeltaOp> {
        &self.aggregator_delta_set
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    /// Materializes this change set: all deltas are converted into writes and
    /// are combined with existing aggregator writes.
    pub fn try_materialize(self, state_view: &impl StateView) -> anyhow::Result<Self, VMStatus> {
        let Self {
            resource_write_set,
            module_write_set,
            mut aggregator_write_set,
            aggregator_delta_set,
            events,
        } = self;

        let into_write =
            |(state_key, delta): (StateKey, DeltaOp)| -> anyhow::Result<(StateKey, WriteOp), VMStatus> {
                let write = delta.try_into_write_op(state_view, &state_key)?;
                Ok((state_key, write))
            };

        let materialized_aggregator_delta_set =
            aggregator_delta_set
                .into_iter()
                .map(into_write)
                .collect::<anyhow::Result<HashMap<StateKey, WriteOp>, VMStatus>>()?;
        aggregator_write_set.extend(materialized_aggregator_delta_set.into_iter());

        Ok(Self {
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            aggregator_delta_set: HashMap::new(),
            events,
        })
    }

    fn squash_additional_aggregator_changes(
        aggregator_write_set: &mut HashMap<StateKey, WriteOp>,
        aggregator_delta_set: &mut HashMap<StateKey, DeltaOp>,
        additional_aggregator_write_set: HashMap<StateKey, WriteOp>,
        additional_aggregator_delta_set: HashMap<StateKey, DeltaOp>,
    ) -> anyhow::Result<(), VMStatus> {
        use WriteOp::*;

        // First, squash deltas.
        for (key, additional_delta_op) in additional_aggregator_delta_set {
            if let Some(write_op) = aggregator_write_set.get_mut(&key) {
                // In this case, delta follows a write op.
                match write_op {
                    Creation(data)
                    | Modification(data)
                    | CreationWithMetadata { data, .. }
                    | ModificationWithMetadata { data, .. } => {
                        // Apply delta on top of creation or modification.
                        let base: u128 = deserialize(data);
                        let value = additional_delta_op
                            .apply_to(base)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                        *data = serialize(&value);
                    },
                    Deletion | DeletionWithMetadata { .. } => {
                        // This case (applying a delta to deleted item) should
                        // never happen. Let's still return an error instead of
                        // panicking.
                        return Err(VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Cannot squash delta which was already deleted."),
                        ));
                    },
                }
            } else {
                // Otherwise, this is a either a new delta or an additional delta
                // for the same state key.
                match aggregator_delta_set.entry(key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming delta
                        // to the existing delta, ensuring the strict ordering.
                        entry
                            .into_mut()
                            .merge_with_next_delta(additional_delta_op)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                    },
                    Vacant(entry) => {
                        // We see this delta for the first time, so simply add it
                        // to the set.
                        entry.insert(additional_delta_op);
                    },
                }
            }
        }

        // Next, squash write ops.
        for (key, additional_write_op) in additional_aggregator_write_set {
            match aggregator_write_set.entry(key) {
                Occupied(mut entry) => {
                    squash_writes_pair!(entry, additional_write_op);
                },
                Vacant(entry) => {
                    // This is a new write op. It can overwrite a delta so we
                    // have to make sure we remove such a delta from the set in
                    // this case.
                    let removed_delta = aggregator_delta_set.remove(entry.key());

                    // We cannot create after modification with a delta!
                    if removed_delta.is_some() && additional_write_op.is_creation() {
                        return Err(VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Cannot create a resource after modification with a delta."),
                        ));
                    }

                    entry.insert(additional_write_op);
                },
            }
        }

        Ok(())
    }

    fn squash_additional_writes(
        write_set: &mut HashMap<StateKey, WriteOp>,
        additional_write_set: HashMap<StateKey, WriteOp>,
    ) -> anyhow::Result<(), VMStatus> {
        for (key, additional_write_op) in additional_write_set.into_iter() {
            match write_set.entry(key) {
                Occupied(mut entry) => {
                    squash_writes_pair!(entry, additional_write_op);
                },
                Vacant(entry) => {
                    entry.insert(additional_write_op);
                },
            }
        }
        Ok(())
    }

    pub fn squash_additional_change_set(
        &mut self,
        additional_change_set: Self,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<(), VMStatus> {
        let Self {
            resource_write_set: additional_resource_write_set,
            module_write_set: additional_module_write_set,
            aggregator_write_set: additional_aggregator_write_set,
            aggregator_delta_set: additional_aggregator_delta_set,
            events: additional_events,
        } = additional_change_set;

        Self::squash_additional_aggregator_changes(
            &mut self.aggregator_write_set,
            &mut self.aggregator_delta_set,
            additional_aggregator_write_set,
            additional_aggregator_delta_set,
        )?;
        Self::squash_additional_writes(
            &mut self.resource_write_set,
            additional_resource_write_set,
        )?;
        Self::squash_additional_writes(&mut self.module_write_set, additional_module_write_set)?;
        self.events.extend(additional_events);

        checker.check_change_set(self)
    }
}

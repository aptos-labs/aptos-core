// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abstract_write_op::{
        AbstractResourceWriteOp, GroupWrite, InPlaceDelayedFieldChangeOp,
        ResourceGroupInPlaceDelayedFieldChangeOp, WriteWithDelayedFieldsOp,
    },
    module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::{ModuleWrite, ModuleWriteSet},
    resolver::ExecutorView,
};
use aptos_aggregator::{
    delayed_change::DelayedChange,
    delta_change_set::{serialize, DeltaOp},
    resolver::AggregatorV1Resolver,
};
use aptos_types::{
    contract_event::ContractEvent,
    error::{code_invariant_error, PanicError},
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        state_value::StateValueMetadata,
    },
    transaction::ChangeSet as StorageChangeSet,
    write_set::{TransactionWrite, WriteOp, WriteOpSize, WriteSetMut},
};
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use rand::Rng;
use std::{
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        BTreeMap, BTreeSet,
    },
    hash::Hash,
    sync::Arc,
};

/// Sporadically checks if the given two input type layouts match.
pub fn randomly_check_layout_matches(
    layout_1: Option<&MoveTypeLayout>,
    layout_2: Option<&MoveTypeLayout>,
) -> Result<(), PanicError> {
    if layout_1.is_some() != layout_2.is_some() {
        return Err(code_invariant_error(format!(
            "Layouts don't match when they are expected to: {:?} and {:?}",
            layout_1, layout_2
        )));
    }
    if layout_1.is_some() {
        // Checking if 2 layouts are equal is a recursive operation and is expensive.
        // We generally call this `randomly_check_layout_matches` function when we know
        // that the layouts are supposed to match. As an optimization, we only randomly
        // check if the layouts are matching.
        let mut rng = rand::thread_rng();
        let random_number: u32 = rng.gen_range(0, 100);
        if random_number == 1 && layout_1 != layout_2 {
            return Err(code_invariant_error(format!(
                "Layouts don't match when they are expected to: {:?} and {:?}",
                layout_1, layout_2
            )));
        }
    }
    Ok(())
}

/// A change set produced by the VM.
///
/// **WARNING**: Just like VMOutput, this type should only be used inside the
/// VM. For storage backends, use `ChangeSet`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VMChangeSet {
    resource_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
    events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,

    // Changes separated out from the writes, for better concurrency,
    // materialized back into resources when transaction output is computed.
    delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,

    // TODO[agg_v1](cleanup) deprecate aggregator_v1 fields.
    aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
    aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
}

macro_rules! squash_writes_pair {
    ($write_entry:ident, $additional_write:ident) => {
        // Squashing creation and deletion is a no-op. In that case, we
        // have to remove the old write op from the write set.
        let noop = !WriteOp::squash($write_entry.get_mut(), $additional_write).map_err(|e| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message(format!("Error while squashing two write ops: {}.", e))
        })?;
        if noop {
            $write_entry.remove();
        }
    };
}

impl VMChangeSet {
    pub fn empty() -> Self {
        Self {
            resource_write_set: BTreeMap::new(),
            events: vec![],
            delayed_field_change_set: BTreeMap::new(),
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
        }
    }

    pub fn new(
        resource_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
    ) -> Self {
        Self {
            resource_write_set,
            events,
            delayed_field_change_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
        }
    }

    // TODO[agg_v2](cleanup) see if we can remove in favor of `new`.
    pub fn new_expanded(
        resource_write_set: BTreeMap<StateKey, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        resource_group_write_set: BTreeMap<StateKey, GroupWrite>,
        aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
        delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        reads_needing_delayed_field_exchange: BTreeMap<
            StateKey,
            (StateValueMetadata, u64, Arc<MoveTypeLayout>),
        >,
        group_reads_needing_delayed_field_exchange: BTreeMap<StateKey, (StateValueMetadata, u64)>,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
    ) -> PartialVMResult<Self> {
        Ok(Self::new(
            resource_write_set
                .into_iter()
                .map::<PartialVMResult<_>, _>(|(k, (w, l))| {
                    Ok((
                        k,
                        AbstractResourceWriteOp::from_resource_write_with_maybe_layout(w, l),
                    ))
                })
                .chain(
                    resource_group_write_set
                        .into_iter()
                        .map(|(k, w)| Ok((k, AbstractResourceWriteOp::WriteResourceGroup(w)))),
                )
                .chain(reads_needing_delayed_field_exchange.into_iter().map(
                    |(k, (metadata, size, layout))| {
                        Ok((
                            k,
                            AbstractResourceWriteOp::InPlaceDelayedFieldChange(
                                InPlaceDelayedFieldChangeOp {
                                    layout,
                                    materialized_size: size,
                                    metadata,
                                },
                            ),
                        ))
                    },
                ))
                .chain(group_reads_needing_delayed_field_exchange.into_iter().map(
                    |(k, (metadata, materialized_size))| {
                        Ok((
                            k,
                            AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(
                                ResourceGroupInPlaceDelayedFieldChangeOp {
                                    metadata,
                                    materialized_size,
                                },
                            ),
                        ))
                    },
                ))
                .try_fold::<_, _, PartialVMResult<BTreeMap<_, _>>>(
                    BTreeMap::new(),
                    |mut acc, element| {
                        let (key, value) = element?;
                        if acc.insert(key, value).is_some() {
                            Err(PartialVMError::new(
                                StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
                            )
                            .with_message(
                                "Found duplicate key across resource change sets.".to_string(),
                            ))
                        } else {
                            Ok(acc)
                        }
                    },
                )?,
            events,
            delayed_field_change_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
        ))
    }

    /// Converts VM-native change set into its storage representation with fully
    /// serialized changes. The conversion fails if:
    /// - deltas are not materialized.
    /// - resource group writes are not (combined &) converted to resource writes.
    /// In addition, the caller can include changes to published modules.
    pub fn try_combine_into_storage_change_set(
        self,
        module_write_set: ModuleWriteSet,
        read_set: BTreeSet<StateKey>,
    ) -> Result<StorageChangeSet, PanicError> {
        // Converting VMChangeSet into TransactionOutput (i.e. storage change set), can
        // be done here only if dynamic_change_set_optimizations have not been used/produced
        // data into the output.
        // If they (DelayedField or ResourceGroup) have added data into the write set, translation
        // into output is more complicated, and needs to be done within BlockExecutor context
        // that knows how to deal with it.
        let Self {
            resource_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
            delayed_field_change_set,
            events,
        } = self;

        if !aggregator_v1_delta_set.is_empty() {
            return Err(code_invariant_error(
                "Cannot convert from VMChangeSet with non-materialized Aggregator V1 deltas to ChangeSet.",
            ));
        }
        if !delayed_field_change_set.is_empty() {
            return Err(code_invariant_error(
                "Cannot convert from VMChangeSet with non-materialized Delayed Field changes to ChangeSet.",
            ));
        }

        let mut write_set_mut = WriteSetMut::default();
        write_set_mut.extend(
            resource_write_set
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k,
                        v.try_into_concrete_write().ok_or_else(|| {
                            code_invariant_error(
                                "Cannot convert from VMChangeSet with non-materialized write set",
                            )
                        })?,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        write_set_mut.extend(module_write_set.into_write_ops());
        write_set_mut.extend(aggregator_v1_write_set);

        let events = events.into_iter().map(|(e, _)| e).collect();
        let write_set = write_set_mut.freeze_with_reads(read_set);
        Ok(StorageChangeSet::new(write_set, events))
    }

    pub fn concrete_write_set_iter(&self) -> impl Iterator<Item = (&StateKey, Option<&WriteOp>)> {
        self.resource_write_set()
            .iter()
            .map(|(k, v)| (k, v.try_as_concrete_write()))
            .chain(
                self.aggregator_v1_write_set()
                    .iter()
                    .map(|(k, v)| (k, Some(v))),
            )
    }

    pub fn resource_write_set(&self) -> &BTreeMap<StateKey, AbstractResourceWriteOp> {
        &self.resource_write_set
    }

    // Called by `into_transaction_output_with_materialized_writes` only.
    pub(crate) fn extend_aggregator_v1_write_set(
        &mut self,
        additional_aggregator_writes: impl Iterator<Item = (StateKey, WriteOp)>,
    ) {
        self.aggregator_v1_write_set
            .extend(additional_aggregator_writes)
    }

    // Called by `into_transaction_output_with_materialized_writes` only.
    pub(crate) fn extend_resource_write_set(
        &mut self,
        materialized_resource_writes: impl Iterator<Item = (StateKey, WriteOp)>,
    ) -> Result<(), PanicError> {
        for (key, new_write) in materialized_resource_writes {
            let abstract_write = self.resource_write_set.get_mut(&key).ok_or_else(|| {
                code_invariant_error(format!(
                    "Cannot patch a resource which does not exist, for: {:?}.",
                    key
                ))
            })?;

            if let AbstractResourceWriteOp::Write(w) = &abstract_write {
                return Err(code_invariant_error(format!(
                    "Trying to patch the value that is already materialized: {:?}: {:?} into {:?}.",
                    key, w, new_write
                )));
            }

            let new_length = new_write.write_op_size().write_len();
            let old_length = abstract_write.materialized_size().write_len();
            if new_length != old_length {
                return Err(code_invariant_error(format!(
                    "Trying to patch the value that changed size during materialization: {:?}: {:?} into {:?}. \nValues {:?} into {:?}.", key, old_length, new_length, abstract_write, new_write,
                )));
            }

            *abstract_write = AbstractResourceWriteOp::Write(new_write);
        }
        Ok(())
    }

    /// The events are set to the input events.
    pub(crate) fn set_events(&mut self, materialized_events: impl Iterator<Item = ContractEvent>) {
        self.events = materialized_events
            .map(|event| (event, None))
            .collect::<Vec<_>>();
    }

    pub(crate) fn drain_delayed_field_change_set(
        &mut self,
    ) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        std::mem::take(&mut self.delayed_field_change_set)
    }

    pub(crate) fn drain_aggregator_v1_delta_set(&mut self) -> BTreeMap<StateKey, DeltaOp> {
        std::mem::take(&mut self.aggregator_v1_delta_set)
    }

    pub fn aggregator_v1_write_set(&self) -> &BTreeMap<StateKey, WriteOp> {
        &self.aggregator_v1_write_set
    }

    pub fn aggregator_v1_delta_set(&self) -> &BTreeMap<StateKey, DeltaOp> {
        &self.aggregator_v1_delta_set
    }

    pub fn delayed_field_change_set(
        &self,
    ) -> &BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        &self.delayed_field_change_set
    }

    pub fn events(&self) -> &[(ContractEvent, Option<MoveTypeLayout>)] {
        &self.events
    }

    /// Materializes this change set: all aggregator v1 deltas are converted into writes and
    /// are combined with existing aggregator writes. The aggregator v2 changeset is not touched.
    pub fn try_materialize_aggregator_v1_delta_set(
        &mut self,
        resolver: &impl AggregatorV1Resolver,
    ) -> VMResult<()> {
        let into_write =
            |(state_key, delta): (StateKey, DeltaOp)| -> VMResult<(StateKey, WriteOp)> {
                // Materialization is needed when committing a transaction, so
                // we need precise mode to compute the true value of an
                // aggregator.
                let write = resolver
                    .try_convert_aggregator_v1_delta_into_write_op(&state_key, &delta)
                    .map_err(|e| {
                        // We need to set abort location for Aggregator V1 to ensure correct VMStatus can
                        // be constructed.
                        const AGGREGATOR_V1_ADDRESS: AccountAddress = CORE_CODE_ADDRESS;
                        const AGGREGATOR_V1_MODULE_NAME: &IdentStr = ident_str!("aggregator");
                        e.finish(Location::Module(ModuleId::new(
                            AGGREGATOR_V1_ADDRESS,
                            AGGREGATOR_V1_MODULE_NAME.into(),
                        )))
                    })?;
                Ok((state_key, write))
            };

        let aggregator_v1_delta_set = std::mem::take(&mut self.aggregator_v1_delta_set);
        let materialized_aggregator_delta_set = aggregator_v1_delta_set
            .into_iter()
            .map(into_write)
            .collect::<VMResult<BTreeMap<StateKey, WriteOp>>>()?;
        self.aggregator_v1_write_set
            .extend(materialized_aggregator_delta_set);
        Ok(())
    }

    fn squash_additional_aggregator_v1_changes(
        aggregator_v1_write_set: &mut BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: &mut BTreeMap<StateKey, DeltaOp>,
        additional_aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        additional_aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
    ) -> PartialVMResult<()> {
        // First, squash deltas.
        for (state_key, additional_delta_op) in additional_aggregator_v1_delta_set {
            if let Some(write_op) = aggregator_v1_write_set.get_mut(&state_key) {
                // In this case, delta follows a write op.
                match write_op.bytes() {
                    Some(bytes) => {
                        // Apply delta on top of creation or modification.
                        // TODO[agg_v1](cleanup): This will not be needed anymore once aggregator
                        // change sets carry non-serialized information.
                        let base: u128 = bcs::from_bytes(bytes)
                            .expect("Deserializing into an aggregator value always succeeds");
                        let value = additional_delta_op
                            .apply_to(base)
                            .map_err(PartialVMError::from)?;
                        write_op.set_bytes(serialize(&value).into())
                    },
                    None => {
                        // This case (applying a delta to deleted item) should
                        // never happen. Let's still return an error instead of
                        // panicking.
                        return Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message(
                            "Cannot squash delta which was already deleted.".to_string(),
                        ));
                    },
                }
            } else {
                // Otherwise, this is a either a new delta or an additional delta
                // for the same state key.
                match aggregator_v1_delta_set.entry(state_key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming delta
                        // to the existing delta, ensuring the strict ordering.
                        entry
                            .into_mut()
                            .merge_with_next_delta(additional_delta_op)
                            .map_err(PartialVMError::from)?;
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
        for (state_key, additional_write_op) in additional_aggregator_v1_write_set {
            match aggregator_v1_write_set.entry(state_key) {
                Occupied(mut entry) => {
                    squash_writes_pair!(entry, additional_write_op);
                },
                Vacant(entry) => {
                    // This is a new write op. It can overwrite a delta so we
                    // have to make sure we remove such a delta from the set in
                    // this case.
                    let removed_delta = aggregator_v1_delta_set.remove(entry.key());

                    // We cannot create after modification with a delta!
                    if removed_delta.is_some() && additional_write_op.is_creation() {
                        return Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message(
                            "Cannot create a resource after modification with a delta.".to_string(),
                        ));
                    }

                    entry.insert(additional_write_op);
                },
            }
        }

        Ok(())
    }

    fn squash_additional_delayed_field_changes(
        change_set: &mut BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        additional_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    ) -> PartialVMResult<()> {
        let merged_changes = additional_change_set
            .into_iter()
            .map(|(id, additional_change)| {
                let prev_change =
                    if let Some(dependent_id) = additional_change.get_merge_dependent_id() {
                        if change_set.contains_key(&id) {
                            return (
                                id,
                                Err(code_invariant_error(format!(
                                "Aggregator change set contains both {:?} and its dependent {:?}",
                                id, dependent_id
                            ))
                                .into()),
                            );
                        }
                        change_set.get(&dependent_id)
                    } else {
                        change_set.get(&id)
                    };
                (
                    id,
                    DelayedChange::merge_two_changes(prev_change, &additional_change),
                )
            })
            .collect::<Vec<_>>();

        for (id, merged_change) in merged_changes.into_iter() {
            change_set.insert(id, merged_change.map_err(PartialVMError::from)?);
        }
        Ok(())
    }

    fn squash_additional_resource_write_ops<
        K: Hash + Eq + PartialEq + Ord + Clone + std::fmt::Debug,
    >(
        write_set: &mut BTreeMap<K, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        additional_write_set: BTreeMap<K, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    ) -> Result<(), PanicError> {
        for (key, additional_entry) in additional_write_set.into_iter() {
            match write_set.entry(key.clone()) {
                Occupied(mut entry) => {
                    // Squash entry and additional entries if type layouts match.
                    let (additional_write_op, additional_type_layout) = additional_entry;
                    let (write_op, type_layout) = entry.get_mut();
                    randomly_check_layout_matches(
                        type_layout.as_deref(),
                        additional_type_layout.as_deref(),
                    )?;
                    let noop = !WriteOp::squash(write_op, additional_write_op).map_err(|e| {
                        code_invariant_error(format!("Error while squashing two write ops: {}.", e))
                    })?;
                    if noop {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    entry.insert(additional_entry);
                },
            }
        }
        Ok(())
    }

    // pub(crate) only for testing
    pub(crate) fn squash_additional_resource_writes(
        write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        additional_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
    ) -> Result<(), PanicError> {
        use AbstractResourceWriteOp::*;
        for (key, additional_entry) in additional_write_set.into_iter() {
            match write_set.entry(key.clone()) {
                Vacant(entry) => {
                    entry.insert(additional_entry);
                },
                Occupied(mut entry) => {
                    let (to_delete, to_overwrite) = match (entry.get_mut(), &additional_entry) {
                        (Write(write_op), Write(additional_write_op)) => {
                            let to_delete = !WriteOp::squash(write_op, additional_write_op.clone())
                                .map_err(|e| {
                                    code_invariant_error(format!(
                                        "Error while squashing two write ops: {}.",
                                        e
                                    ))
                                })?;
                            (to_delete, false)
                        },
                        (
                            WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                                write_op,
                                layout,
                                materialized_size,
                            }),
                            WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                                write_op: additional_write_op,
                                layout: additional_layout,
                                materialized_size: additional_materialized_size,
                            }),
                        ) => {
                            randomly_check_layout_matches(Some(layout), Some(additional_layout))?;
                            let to_delete = !WriteOp::squash(write_op, additional_write_op.clone())
                                .map_err(|e| {
                                    code_invariant_error(format!(
                                        "Error while squashing two write ops: {}.",
                                        e
                                    ))
                                })?;
                            *materialized_size = *additional_materialized_size;
                            (to_delete, false)
                        },
                        (
                            WriteResourceGroup(group),
                            WriteResourceGroup(GroupWrite {
                                metadata_op: additional_metadata_op,
                                inner_ops: additional_inner_ops,
                                maybe_group_op_size: additional_maybe_group_op_size,
                                prev_group_size: _, // n.b. group.prev_group_size deliberately kept as is
                            }),
                        ) => {
                            // Squashing creation and deletion is a no-op. In that case, we have to
                            // remove the old GroupWrite from the group write set.
                            let to_delete = !WriteOp::squash(
                                &mut group.metadata_op,
                                additional_metadata_op.clone(),
                            )
                            .map_err(|e| {
                                code_invariant_error(format!(
                                    "Error while squashing two group write metadata ops: {}.",
                                    e
                                ))
                            })?;
                            if to_delete {
                                (true, false)
                            } else {
                                Self::squash_additional_resource_write_ops(
                                    &mut group.inner_ops,
                                    additional_inner_ops.clone(),
                                )?;

                                group.maybe_group_op_size = *additional_maybe_group_op_size;

                                //
                                // n.b. group.prev_group_size deliberately kept as is
                                //

                                (false, false)
                            }
                        },
                        (
                            WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                                materialized_size,
                                ..
                            }),
                            InPlaceDelayedFieldChange(InPlaceDelayedFieldChangeOp {
                                materialized_size: additional_materialized_size,
                                ..
                            }),
                        ) => {
                            // Read cannot change the size (i.e. delayed fields don't modify size)
                            if materialized_size != &Some(*additional_materialized_size) {
                                return Err(code_invariant_error(format!(
                                    "Trying to squash writes where read has different size: {:?}: {:?}",
                                    materialized_size,
                                    additional_materialized_size
                                )));
                            }
                            // any newer read should've read the original write and contain all info from it
                            (false, false)
                        },
                        (
                            WriteResourceGroup(GroupWrite {
                                maybe_group_op_size: materialized_size,
                                ..
                            }),
                            ResourceGroupInPlaceDelayedFieldChange(
                                ResourceGroupInPlaceDelayedFieldChangeOp {
                                    materialized_size: additional_materialized_size,
                                    ..
                                },
                            ),
                        ) => {
                            // Read cannot change the size (i.e. delayed fields don't modify size)
                            if materialized_size.map(|v| v.get())
                                != Some(*additional_materialized_size)
                            {
                                return Err(code_invariant_error(format!(
                                    "Trying to squash group writes where read has different size: {:?}: {:?}",
                                    materialized_size,
                                    additional_materialized_size
                                )));
                            }
                            // any newer read should've read the original write and contain all info from it
                            (false, false)
                        },
                        // If previous value is a read, newer value overwrites it
                        (
                            InPlaceDelayedFieldChange(_),
                            WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_),
                        )
                        | (
                            ResourceGroupInPlaceDelayedFieldChange(_),
                            WriteResourceGroup(_) | ResourceGroupInPlaceDelayedFieldChange(_),
                        ) => (false, true),
                        (
                            Write(_),
                            WriteWithDelayedFields(_)
                            | WriteResourceGroup(_)
                            | InPlaceDelayedFieldChange(_)
                            | ResourceGroupInPlaceDelayedFieldChange(_),
                        )
                        | (
                            WriteWithDelayedFields(_),
                            Write(_)
                            | WriteResourceGroup(_)
                            | ResourceGroupInPlaceDelayedFieldChange(_),
                        )
                        | (
                            WriteResourceGroup(_),
                            Write(_) | WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_),
                        )
                        | (
                            InPlaceDelayedFieldChange(_),
                            Write(_)
                            | WriteResourceGroup(_)
                            | ResourceGroupInPlaceDelayedFieldChange(_),
                        )
                        | (
                            ResourceGroupInPlaceDelayedFieldChange(_),
                            Write(_) | WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_),
                        ) => {
                            return Err(code_invariant_error(format!(
                                "Trying to squash incompatible writes: {:?}: {:?} into {:?}.",
                                entry.key(),
                                entry.get(),
                                additional_entry
                            )));
                        },
                    };

                    if to_overwrite {
                        entry.insert(additional_entry);
                    } else if to_delete {
                        entry.remove();
                    }
                },
            }
        }
        Ok(())
    }

    pub fn squash_additional_change_set(
        &mut self,
        additional_change_set: Self,
    ) -> PartialVMResult<()> {
        let Self {
            resource_write_set: additional_resource_write_set,
            aggregator_v1_write_set: additional_aggregator_write_set,
            aggregator_v1_delta_set: additional_aggregator_delta_set,
            delayed_field_change_set: additional_delayed_field_change_set,
            events: additional_events,
        } = additional_change_set;

        Self::squash_additional_aggregator_v1_changes(
            &mut self.aggregator_v1_write_set,
            &mut self.aggregator_v1_delta_set,
            additional_aggregator_write_set,
            additional_aggregator_delta_set,
        )?;
        Self::squash_additional_resource_writes(
            &mut self.resource_write_set,
            additional_resource_write_set,
        )?;
        Self::squash_additional_delayed_field_changes(
            &mut self.delayed_field_change_set,
            additional_delayed_field_change_set,
        )?;
        self.events.extend(additional_events);
        Ok(())
    }

    pub fn has_creation(&self) -> bool {
        self.write_set_size_iter()
            .any(|(_key, op_size)| matches!(op_size, WriteOpSize::Creation { .. }))
    }
}

/// Builds a new change set from the storage representation.
///
/// **WARNING**: this creates a write set that assumes dynamic change set optimizations to be disabled.
/// this needs to be applied directly to storage, you cannot get appropriate reads from this in a
/// dynamic change set optimization enabled context.
/// We have two dynamic change set optimizations, both there to reduce conflicts between transactions:
///  - exchanging delayed fields and leaving their materialization to happen at the end
///  - unpacking resource groups and treating each resource inside it separately
///
/// **WARNING**: Has complexity O(#write_ops) because we need to iterate
/// over blobs and split them into resources or modules. Only used to
/// support transactions with write-set payload.
///
/// Note: does not separate out individual resource group updates.
pub fn create_vm_change_set_with_module_write_set_when_delayed_field_optimization_disabled(
    change_set: StorageChangeSet,
) -> (VMChangeSet, ModuleWriteSet) {
    let (write_set, events) = change_set.into_inner();

    // There should be no aggregator writes if we have a change set from
    // storage.
    let mut resource_write_set = BTreeMap::new();
    let mut module_write_ops = BTreeMap::new();

    for (state_key, write_op) in write_set.expect_into_write_op_iter() {
        if let StateKeyInner::AccessPath(ap) = state_key.inner() {
            if let Some(module_id) = ap.try_get_module_id() {
                module_write_ops.insert(state_key, ModuleWrite::new(module_id, write_op));
                continue;
            }
        }

        // TODO[agg_v1](fix) While everything else must be a resource, first
        // version of aggregators is implemented as a table item. Revisit when
        // we split MVHashMap into data and aggregators.

        // We can set layout to None, as we are not in the is_delayed_field_optimization_capable context
        resource_write_set.insert(state_key, AbstractResourceWriteOp::Write(write_op));
    }

    // We can set layout to None, as we are not in the is_delayed_field_optimization_capable context
    let events = events.into_iter().map(|event| (event, None)).collect();
    let change_set = VMChangeSet::new(
        resource_write_set,
        events,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );

    let module_write_set = ModuleWriteSet::new(module_write_ops);
    (change_set, module_write_set)
}

pub struct WriteOpInfo<'a> {
    pub key: &'a StateKey,
    pub op_size: WriteOpSize,
    pub prev_size: u64,
    pub metadata_mut: &'a mut StateValueMetadata,
}

/// Represents the main functionality of any change set representation:
///   1. It must contain write ops, and allow iterating over their sizes,
///      as well as other information.
///   2. it must also contain events.
pub trait ChangeSetInterface {
    fn num_write_ops(&self) -> usize;

    fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)>;

    fn events_iter(&self) -> impl Iterator<Item = &ContractEvent>;

    fn write_op_info_iter_mut<'a>(
        &'a mut self,
        executor_view: &'a dyn ExecutorView,
        module_storage: &'a impl AptosModuleStorage,
        fix_prev_materialized_size: bool,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo<'a>>>;
}

impl ChangeSetInterface for VMChangeSet {
    fn num_write_ops(&self) -> usize {
        // Note: we only use resources and aggregators because they use write ops directly,
        // and deltas & events are not part of these.
        self.resource_write_set().len() + self.aggregator_v1_write_set().len()
    }

    fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.resource_write_set()
            .iter()
            .map(|(k, v)| (k, v.materialized_size()))
            .chain(
                self.aggregator_v1_write_set()
                    .iter()
                    .map(|(k, v)| (k, v.write_op_size())),
            )
    }

    fn write_op_info_iter_mut<'a>(
        &'a mut self,
        executor_view: &'a dyn ExecutorView,
        _module_storage: &'a impl AptosModuleStorage,
        fix_prev_materialized_size: bool,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo<'a>>> {
        let resources = self.resource_write_set.iter_mut().map(move |(key, op)| {
            Ok(WriteOpInfo {
                key,
                op_size: op.materialized_size(),
                prev_size: op.prev_materialized_size(
                    key,
                    executor_view,
                    fix_prev_materialized_size,
                )?,
                metadata_mut: op.metadata_mut(),
            })
        });
        let v1_aggregators = self.aggregator_v1_write_set.iter_mut().map(|(key, op)| {
            Ok(WriteOpInfo {
                key,
                op_size: op.write_op_size(),
                prev_size: executor_view
                    .get_aggregator_v1_state_value_size(key)?
                    .unwrap_or(0),
                metadata_mut: op.metadata_mut(),
            })
        });

        resources.chain(v1_aggregators)
    }

    fn events_iter(&self) -> impl Iterator<Item = &ContractEvent> {
        self.events().iter().map(|(e, _)| e)
    }
}

// Tests are in test_change_set.rs.

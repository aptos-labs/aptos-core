// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    abstract_write_op::{
        AbstractResourceWriteOp, GroupWrite, InPlaceDelayedFieldChangeOp,
        ResourceGroupInPlaceDelayedFieldChangeOp, WriteWithDelayedFieldsOp,
    },
    module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::{ModuleWrite, ModuleWriteSet},
    resolver::ExecutorView,
};
use aptos_aggregator::delayed_change::DelayedChange;
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
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use rand::Rng;
use std::{
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        BTreeMap,
    },
    hash::Hash,
};
use triomphe::Arc as TriompheArc;

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
}

impl VMChangeSet {
    pub fn empty() -> Self {
        Self {
            resource_write_set: BTreeMap::new(),
            events: vec![],
            delayed_field_change_set: BTreeMap::new(),
        }
    }

    pub fn new(
        resource_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    ) -> Self {
        Self {
            resource_write_set,
            events,
            delayed_field_change_set,
        }
    }

    // TODO[agg_v2](cleanup) see if we can remove in favor of `new`.
    pub fn new_expanded(
        resource_write_set: BTreeMap<StateKey, (WriteOp, Option<TriompheArc<MoveTypeLayout>>)>,
        resource_group_write_set: BTreeMap<StateKey, GroupWrite>,
        aggregator_v1_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
        delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        reads_needing_delayed_field_exchange: BTreeMap<
            StateKey,
            (StateValueMetadata, u64, TriompheArc<MoveTypeLayout>),
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
                                    is_aggregator_v1_delta: false,
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
                .chain(
                    aggregator_v1_write_set
                        .into_iter()
                        .map(|(k, op)| Ok((k, op))),
                )
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
    ) -> Result<StorageChangeSet, PanicError> {
        // Converting VMChangeSet into TransactionOutput (i.e. storage change set), can
        // be done here only if dynamic_change_set_optimizations have not been used/produced
        // data into the output.
        // If they (DelayedField or ResourceGroup) have added data into the write set, translation
        // into output is more complicated, and needs to be done within BlockExecutor context
        // that knows how to deal with it.
        let Self {
            resource_write_set,
            delayed_field_change_set,
            events,
        } = self;

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

        let events = events.into_iter().map(|(e, _)| e).collect();
        let write_set = write_set_mut
            .freeze()
            .expect("Freezing a WriteSet does not fail.");
        Ok(StorageChangeSet::new(write_set, events))
    }

    pub fn concrete_write_set_iter(&self) -> impl Iterator<Item = (&StateKey, Option<&WriteOp>)> {
        self.resource_write_set()
            .iter()
            .map(|(k, v)| (k, v.try_as_concrete_write()))
    }

    pub fn resource_write_set(&self) -> &BTreeMap<StateKey, AbstractResourceWriteOp> {
        &self.resource_write_set
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

            if let AbstractResourceWriteOp::Write(w, _) = &abstract_write {
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

            *abstract_write = AbstractResourceWriteOp::Write(new_write, false);
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

    pub fn delayed_field_change_set(
        &self,
    ) -> &BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        &self.delayed_field_change_set
    }

    pub fn events(&self) -> &[(ContractEvent, Option<MoveTypeLayout>)] {
        &self.events
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
        write_set: &mut BTreeMap<K, (WriteOp, Option<TriompheArc<MoveTypeLayout>>)>,
        additional_write_set: BTreeMap<K, (WriteOp, Option<TriompheArc<MoveTypeLayout>>)>,
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
        // When true, a full write (resource group or standalone delayed-field
        // resource) followed by an in-place delayed-field change on the same
        // key is rejected outright instead of being allowed when sizes match.
        strict_delayed_field_squash: bool,
    ) -> Result<(), PanicError> {
        use AbstractResourceWriteOp::*;
        for (key, additional_entry) in additional_write_set.into_iter() {
            match write_set.entry(key.clone()) {
                Vacant(entry) => {
                    entry.insert(additional_entry);
                },
                Occupied(mut entry) => {
                    let (to_delete, to_overwrite) = match (entry.get_mut(), &additional_entry) {
                        (
                            Write(write_op, is_aggregator_v1_delta),
                            Write(additional_write_op, additional_is_aggregator_v1_delta),
                        ) => {
                            let to_delete = !WriteOp::squash(write_op, additional_write_op.clone())
                                .map_err(|e| {
                                    code_invariant_error(format!(
                                        "Error while squashing two write ops: {}.",
                                        e
                                    ))
                                })?;
                            // The squashed write stays a V1 aggregator delta only if both
                            // writes are deltas. A read or create anywhere in the chain
                            // promotes it to a normal charged write, regardless of order.
                            *is_aggregator_v1_delta =
                                *is_aggregator_v1_delta && *additional_is_aggregator_v1_delta;
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
                                is_aggregator_v1_delta,
                                ..
                            }),
                        ) => {
                            if strict_delayed_field_squash && !is_aggregator_v1_delta {
                                // SAFETY (fail-closed window [RELEASE_V1_46, RELEASE_V1_48)): the
                                // standalone-resource analogue of the resource-group arm below. A
                                // full resource write squashed with a later in-place delayed-field
                                // exchange on the same key is rejected; a matching materialized
                                // size does not prove the later exchange reconciles with what the
                                // earlier session wrote. From RELEASE_V1_48 the view-layer fix makes
                                // this sound again and we fall through to the legacy merge.
                                // An only exemption is V1 aggregator. Its item is u128 scalar,
                                // and so previous write only carries that change. As a result,
                                // squashing is allowed because the delta folds into u128 exactly.
                                return Err(code_invariant_error(format!(
                                    "Refusing to squash a resource write with a later in-place \
                                     delayed-field change on the same key (fail-closed for safety): \
                                     {:?} into {:?}.",
                                    key, additional_entry
                                )));
                            }
                            // Legacy behavior (outside the fail-closed window): a read cannot
                            // change the size (i.e. delayed fields don't modify size), so allow the
                            // merge only when the materialized sizes match.
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
                            if strict_delayed_field_squash {
                                // SAFETY (fail-closed window [RELEASE_V1_46, RELEASE_V1_48)): we
                                // deliberately do NOT allow squashing a full resource-group write
                                // (an earlier session that wrote the group, e.g. structurally
                                // changed its membership) with a later in-place delayed-field
                                // exchange on the same group (e.g. the gas epilogue touching an
                                // aggregator in that group). A matching materialized size does not
                                // prove the later session's delayed-field exchange reconciles with
                                // the group the earlier session wrote, so rather than risk a silent
                                // mis-merge of a resource group (which holds asset balances), we
                                // abort the transaction. This is asset-safe: no change set is
                                // applied. Legitimate flows touch the group purely in place
                                // (ResourceGroupInPlaceDelayedFieldChange on both sides), handled by
                                // the arm below. From RELEASE_V1_48 the view-layer fix makes this
                                // sound again and we fall through to the legacy merge.
                                return Err(code_invariant_error(format!(
                                    "Refusing to squash a resource-group write with a later \
                                     in-place delayed-field change on the same group (fail-closed \
                                     for safety): {:?} into {:?}.",
                                    key, additional_entry
                                )));
                            }
                            // Legacy behavior (outside the fail-closed window): a read cannot
                            // change the size (i.e. delayed fields don't modify size), so allow the
                            // merge only when the materialized sizes match.
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
                            Write(..),
                            WriteWithDelayedFields(_)
                            | WriteResourceGroup(_)
                            | InPlaceDelayedFieldChange(_)
                            | ResourceGroupInPlaceDelayedFieldChange(_),
                        )
                        | (
                            WriteWithDelayedFields(_),
                            Write(..)
                            | WriteResourceGroup(_)
                            | ResourceGroupInPlaceDelayedFieldChange(_),
                        )
                        | (
                            WriteResourceGroup(_),
                            Write(..) | WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_),
                        )
                        | (
                            InPlaceDelayedFieldChange(_),
                            Write(..)
                            | WriteResourceGroup(_)
                            | ResourceGroupInPlaceDelayedFieldChange(_),
                        )
                        | (
                            ResourceGroupInPlaceDelayedFieldChange(_),
                            Write(..) | WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_),
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
        strict_delayed_field_squash: bool,
    ) -> PartialVMResult<()> {
        let Self {
            resource_write_set: additional_resource_write_set,
            delayed_field_change_set: additional_delayed_field_change_set,
            events: additional_events,
        } = additional_change_set;

        Self::squash_additional_resource_writes(
            &mut self.resource_write_set,
            additional_resource_write_set,
            strict_delayed_field_squash,
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
        resource_write_set.insert(state_key, AbstractResourceWriteOp::Write(write_op, false));
    }

    // We can set layout to None, as we are not in the is_delayed_field_optimization_capable context
    let events = events.into_iter().map(|event| (event, None)).collect();
    let change_set = VMChangeSet::new(resource_write_set, events, BTreeMap::new());

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
        // Only resources are counted (legacy aggregator V1 deltas excluded).
        self.resource_write_set()
            .values()
            .filter(|v| !v.is_aggregator_v1_delta())
            .count()
    }

    fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.resource_write_set()
            .iter()
            .filter(|(_, v)| {
                // Legacy aggregator V1 deltas excluded. They were never part of
                // the write set iterator.
                !v.is_aggregator_v1_delta()
            })
            .map(|(k, v)| (k, v.materialized_size()))
    }

    fn write_op_info_iter_mut<'a>(
        &'a mut self,
        executor_view: &'a dyn ExecutorView,
        _module_storage: &'a impl AptosModuleStorage,
        fix_prev_materialized_size: bool,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo<'a>>> {
        self.resource_write_set
            .iter_mut()
            .filter(|(_, op)| {
                // Legacy aggregator V1 deltas excluded. They were never part of
                // the write set iterator.
                !op.is_aggregator_v1_delta()
            })
            .map(move |(key, op)| {
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
            })
    }

    fn events_iter(&self) -> impl Iterator<Item = &ContractEvent> {
        self.events().iter().map(|(e, _)| e)
    }
}

// Tests are in test_change_set.rs.

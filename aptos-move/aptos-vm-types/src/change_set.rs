// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::check_change_set::CheckChangeSet;
use aptos_aggregator::{
    delayed_change::DelayedChange,
    delta_change_set::{serialize, DeltaOp},
    resolver::AggregatorV1Resolver,
    types::{code_invariant_error, DelayedFieldID},
};
use aptos_types::{
    aggregator::PanicError,
    contract_event::ContractEvent,
    state_store::{
        state_key::{StateKey, StateKeyInner},
        state_value::StateValueMetadata,
    },
    transaction::ChangeSet as StorageChangeSet,
    write_set::{TransactionWrite, WriteOp, WriteOpSize, WriteSetMut},
};
use claims::assert_none;
use move_binary_format::errors::{Location, PartialVMError};
use move_core_types::{
    language_storage::StructTag,
    value::MoveTypeLayout,
    vm_status::{err_msg, StatusCode, VMStatus},
};
use std::{
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        BTreeMap,
    },
    hash::Hash,
    sync::Arc,
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AbstractResourceWriteOp {
    Write(WriteOp),
    WriteWithDelayedFields(WriteWithDelayedFieldsOp),
    // Prior to adding a dedicated write-set for resource groups, all resource group
    // updates are merged into a single WriteOp included in the resource_write_set.
    WriteResourceGroup(GroupWrite),
    // No writes in the resource, except for delayed field changes.
    InPlaceDelayedFieldChange(InPlaceDelayedFieldChangeOp),
    // No writes in the resource group, except for delayed field changes.
    ResourceGroupInPlaceDelayedFieldChange(ResourceGroupInPlaceDelayedFieldChangeOp),
}

impl AbstractResourceWriteOp {
    pub fn try_as_concrete_write(&self) -> Option<&WriteOp> {
        if let AbstractResourceWriteOp::Write(write_op) = self {
            Some(write_op)
        } else {
            None
        }
    }

    pub fn try_into_concrete_write(self) -> Option<WriteOp> {
        if let AbstractResourceWriteOp::Write(write_op) = self {
            Some(write_op)
        } else {
            None
        }
    }

    pub fn materialized_size(&self) -> WriteOpSize {
        use AbstractResourceWriteOp::*;
        match self {
            Write(write) => write.into(),
            WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                write_op,
                materialized_size,
                ..
            })
            | WriteResourceGroup(GroupWrite {
                metadata_op: write_op,
                maybe_group_op_size: materialized_size,
                ..
            }) => {
                use WriteOp::*;
                match write_op {
                    Creation(_) | CreationWithMetadata { .. } => WriteOpSize::Creation {
                        write_len: materialized_size.unwrap(),
                    },
                    Modification(_) | ModificationWithMetadata { .. } => {
                        WriteOpSize::Modification {
                            write_len: materialized_size.unwrap(),
                        }
                    },
                    Deletion => WriteOpSize::Deletion,
                    DeletionWithMetadata { metadata } => WriteOpSize::DeletionWithDeposit {
                        deposit: metadata.deposit(),
                    },
                }
            },
            InPlaceDelayedFieldChange(InPlaceDelayedFieldChangeOp {
                materialized_size, ..
            })
            | ResourceGroupInPlaceDelayedFieldChange(ResourceGroupInPlaceDelayedFieldChangeOp {
                materialized_size,
                ..
            }) => WriteOpSize::Modification {
                write_len: *materialized_size,
            },
        }
    }

    pub fn get_creation_metadata_mut(&mut self) -> Option<&mut StateValueMetadata> {
        use AbstractResourceWriteOp::*;
        match self {
            Write(WriteOp::CreationWithMetadata { metadata, .. })
            | WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                write_op: WriteOp::CreationWithMetadata { metadata, .. },
                ..
            })
            | WriteResourceGroup(GroupWrite {
                metadata_op: WriteOp::CreationWithMetadata { metadata, .. },
                ..
            }) => Some(metadata),
            _ => None,
        }
    }
}

/// Describes an update to a resource group granularly, with WriteOps to affected
/// member resources of the group, as well as a separate WriteOp for metadata and size.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct GroupWrite {
    /// Op of the correct kind (creation / modification / deletion) and metadata, and
    /// the size of the group after the updates encoded in the bytes (no bytes for
    /// deletion). Relevant during block execution, where the information read to
    /// derive metadata_op will be validated during parallel execution to make sure
    /// it is correct, and the bytes will be replaced after the transaction is committed
    /// with correct serialized group update to obtain storage WriteOp.
    pub metadata_op: WriteOp,
    /// Updates to individual group members. WriteOps are 'legacy', i.e. no metadata.
    /// If the metadata_op is a deletion, all (correct) inner_ops should be deletions,
    /// and if metadata_op is a creation, then there may not be a creation inner op.
    /// Not vice versa, e.g. for deleted inner ops, other untouched resources may still
    /// exist in the group. Note: During parallel block execution, due to speculative
    /// reads, this invariant may be violated (and lead to speculation error if observed)
    /// but guaranteed to fail validation and lead to correct re-execution in that case.
    inner_ops: BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    /// Group size as used for gas charging, None if (metadata_)op is Deletion.
    maybe_group_op_size: Option<u64>,
}

impl GroupWrite {
    /// Creates a group write and ensures that the format is correct: in particular,
    /// sets the bytes of a non-deletion metadata_op by serializing the provided size,
    /// and ensures inner ops do not contain any metadata.
    pub fn new(
        metadata_op: WriteOp,
        inner_ops: Vec<(StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>))>,
        group_size: u64,
    ) -> Self {
        assert!(
            metadata_op.bytes().is_none() || metadata_op.bytes().unwrap().is_empty(),
            "Metadata op should have empty bytes. metadata_op: {:?}",
            metadata_op
        );
        for (_tag, (v, _layout)) in &inner_ops {
            assert_none!(v.metadata(), "Group inner ops must have no metadata");
        }

        let maybe_group_op_size = (!metadata_op.is_deletion()).then_some(group_size);

        Self {
            metadata_op,
            // TODO[agg_v2](optimize): We are using BTreeMap and Vec in different places to
            // store resources in resources groups. Inefficient to convert the datastructures
            // back and forth. Need to optimize this.
            inner_ops: inner_ops.into_iter().collect(),
            maybe_group_op_size,
        }
    }

    /// Utility method that extracts the serialized group size from metadata_op. Returns
    /// None if group is being deleted, otherwise asserts on deserializing the size.
    pub fn maybe_group_op_size(&self) -> Option<u64> {
        self.maybe_group_op_size
    }

    // TODO: refactor storage fee & refund interfaces to operate on metadata directly,
    // as that would avoid providing &mut to the whole metadata op in here, including
    // bytes that are not raw bytes (encoding group size) and must not be modified.
    pub fn metadata_op_mut(&mut self) -> &mut WriteOp {
        &mut self.metadata_op
    }

    pub fn metadata_op(&self) -> &WriteOp {
        &self.metadata_op
    }

    pub fn inner_ops(&self) -> &BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)> {
        &self.inner_ops
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct WriteWithDelayedFieldsOp {
    pub write_op: WriteOp,
    pub layout: Arc<MoveTypeLayout>,
    pub materialized_size: Option<u64>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct InPlaceDelayedFieldChangeOp {
    pub layout: Arc<MoveTypeLayout>,
    pub materialized_size: u64,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ResourceGroupInPlaceDelayedFieldChangeOp {
    pub metadata_op: WriteOp,
    pub materialized_size: u64,
}

/// A change set produced by the VM.
///
/// **WARNING**: Just like VMOutput, this type should only be used inside the
/// VM. For storage backends, use `ChangeSet`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VMChangeSet {
    resource_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
    module_write_set: BTreeMap<StateKey, WriteOp>,
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
            resource_write_set: BTreeMap::new(),
            module_write_set: BTreeMap::new(),
            events: vec![],
            delayed_field_change_set: BTreeMap::new(),
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
        }
    }

    pub fn new(
        resource_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
        module_write_set: BTreeMap<StateKey, WriteOp>,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
        checker: &dyn CheckChangeSet,
    ) -> Result<Self, VMStatus> {
        let change_set = Self {
            resource_write_set,
            module_write_set,
            events,
            delayed_field_change_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
        };
        // Returns an error if structure of the change set is not valid,
        // e.g. the size in bytes is too large.
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    // TODO[agg_v2](cleanup) see if we can remove in favor of `new`.
    pub fn new_expanded(
        resource_write_set: BTreeMap<StateKey, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        resource_group_write_set: BTreeMap<StateKey, GroupWrite>,
        module_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
        delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        reads_needing_delayed_field_exchange: BTreeMap<StateKey, (WriteOp, Arc<MoveTypeLayout>)>,
        group_reads_needing_delayed_field_exchange: BTreeMap<StateKey, (WriteOp, u64)>,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        checker: &dyn CheckChangeSet,
    ) -> Result<Self, VMStatus> {
        Self::new(
            resource_write_set
                .into_iter()
                .map(|(k, (w, l))| {
                    Ok((
                        k,
                        if let Some(layout) = l {
                            let materialized_size = WriteOpSize::from(&w).write_len();
                            AbstractResourceWriteOp::WriteWithDelayedFields(
                                WriteWithDelayedFieldsOp {
                                    write_op: w,
                                    layout,
                                    materialized_size,
                                },
                            )
                        } else {
                            AbstractResourceWriteOp::Write(w)
                        },
                    ))
                })
                .chain(
                    resource_group_write_set
                        .into_iter()
                        .map(|(k, w)| Ok((k, AbstractResourceWriteOp::WriteResourceGroup(w)))),
                )
                .chain(
                    reads_needing_delayed_field_exchange
                        .into_iter()
                        .map(|(k, (w, layout))| {
                            Ok((
                                k,
                                AbstractResourceWriteOp::InPlaceDelayedFieldChange(
                                    InPlaceDelayedFieldChangeOp {
                                        layout,
                                        materialized_size: WriteOpSize::from(&w)
                                            .write_len()
                                            .ok_or_else(|| {
                                                VMStatus::error(
                                                    StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR,
                                                    err_msg(
                                                        "Read with exchange cannot be a delete.",
                                                    ),
                                                )
                                            })?,
                                    },
                                ),
                            ))
                        }),
                )
                .chain(group_reads_needing_delayed_field_exchange.into_iter().map(
                    |(k, (metadata_op, materialized_size))| {
                        Ok((
                            k,
                            AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(
                                ResourceGroupInPlaceDelayedFieldChangeOp {
                                    metadata_op,
                                    materialized_size,
                                },
                            ),
                        ))
                    },
                ))
                .collect::<Result<BTreeMap<_, _>, VMStatus>>()?,
            module_write_set,
            events,
            delayed_field_change_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
            checker,
        )
    }

    /// Builds a new change set from the storage representation.
    ///
    /// **WARNING**: Has complexity O(#write_ops) because we need to iterate
    /// over blobs and split them into resources or modules. Only used to
    /// support transactions with write-set payload.
    ///
    /// Note: does not separate out individual resource group updates.
    pub fn try_from_storage_change_set(
        change_set: StorageChangeSet,
        checker: &dyn CheckChangeSet,
        // Pass in within which resolver context are we creating this change set.
        // Used to eagerly reject changes created in an incompatible way.
        is_delayed_field_optimization_capable: bool,
    ) -> anyhow::Result<Self, VMStatus> {
        assert!(
            !is_delayed_field_optimization_capable,
            "try_from_storage_change_set can only be called in non-is_delayed_field_optimization_capable context, as it doesn't support delayed field changes (type layout) and resource groups");

        let (write_set, events) = change_set.into_inner();

        // There should be no aggregator writes if we have a change set from
        // storage.
        let mut resource_write_set = BTreeMap::new();
        let mut module_write_set = BTreeMap::new();

        for (state_key, write_op) in write_set {
            if matches!(state_key.inner(), StateKeyInner::AccessPath(ap) if ap.is_code()) {
                module_write_set.insert(state_key, write_op);
            } else {
                // TODO[agg_v1](fix) While everything else must be a resource, first
                // version of aggregators is implemented as a table item. Revisit when
                // we split MVHashMap into data and aggregators.

                // We can set layout to None, as we are not in the is_delayed_field_optimization_capable context
                resource_write_set.insert(state_key, AbstractResourceWriteOp::Write(write_op));
            }
        }

        // We can set layout to None, as we are not in the is_delayed_field_optimization_capable context
        let events = events.into_iter().map(|event| (event, None)).collect();
        let change_set = Self {
            // TODO[agg_v2](fix): do we use same or different capable flag for resource groups?
            // We should skip unpacking resource groups, as we are not in the is_delayed_field_optimization_capable
            // context (i.e. if dynamic_change_set_optimizations_enabled is disabled)
            resource_write_set,
            module_write_set,
            delayed_field_change_set: BTreeMap::new(),
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
            events,
        };
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    /// Converts VM-native change set into its storage representation with fully
    /// serialized changes. The conversion fails if:
    /// - deltas are not materialized.
    /// - resource group writes are not (combined &) converted to resource writes.
    pub fn try_into_storage_change_set(self) -> Result<StorageChangeSet, PanicError> {
        // Converting VMChangeSet into TransactionOutput (i.e. storage change set), can
        // be done here only if dynamic_change_set_optimizations have not been used/produced
        // data into the output.
        // If they (DelayedField or ResourceGroup) have added data into the write set, translation
        // into output is more complicated, and needs to be done within BlockExecutor context
        // that knows how to deal with it.
        let Self {
            resource_write_set,
            module_write_set,
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
        write_set_mut.extend(module_write_set);
        write_set_mut.extend(aggregator_v1_write_set);

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
            .chain(
                self.module_write_set()
                    .iter()
                    .chain(self.aggregator_v1_write_set().iter())
                    .map(|(k, v)| (k, Some(v))),
            )
    }

    pub fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.resource_write_set()
            .iter()
            .map(|(k, v)| (k, v.materialized_size()))
            .chain(
                self.module_write_set()
                    .iter()
                    .chain(self.aggregator_v1_write_set().iter())
                    .map(|(k, v)| (k, WriteOpSize::from(v))),
            )
    }

    pub fn num_write_ops(&self) -> usize {
        self.resource_write_set().len()
            + self.module_write_set().len()
            + self.aggregator_v1_write_set().len()
    }

    pub fn write_set_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&StateKey, WriteOpSize, Option<&mut StateValueMetadata>)> {
        self.resource_write_set
            .iter_mut()
            .map(|(k, v)| (k, v.materialized_size(), v.get_creation_metadata_mut()))
            .chain(
                self.module_write_set
                    .iter_mut()
                    .chain(self.aggregator_v1_write_set.iter_mut())
                    .map(|(k, v)| {
                        (k, WriteOpSize::from(v as &WriteOp), match v {
                            WriteOp::CreationWithMetadata { metadata, .. } => Some(metadata),
                            _ => None,
                        })
                    }),
            )
    }

    pub fn resource_write_set(&self) -> &BTreeMap<StateKey, AbstractResourceWriteOp> {
        &self.resource_write_set
    }

    pub fn module_write_set(&self) -> &BTreeMap<StateKey, WriteOp> {
        &self.module_write_set
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
        patched_resource_writes: impl Iterator<Item = (StateKey, WriteOp)>,
    ) -> Result<(), PanicError> {
        for (k, new_write) in patched_resource_writes {
            match self.resource_write_set.entry(k) {
                Vacant(v) => {
                    return Err(code_invariant_error(format!(
                        "Cannot patch a resource which does not exist, for: {:?}.",
                        v.key()
                    )));
                },
                Occupied(mut o) => {
                    if let AbstractResourceWriteOp::Write(w) = o.get() {
                        return Err(code_invariant_error(format!(
                            "Trying to patch the value that is already materialized: {:?}: {:?} into {:?}.", o.key(), w, new_write
                        )));
                    }

                    let new_length = WriteOpSize::from(&new_write).write_len();
                    let old_length = o.get().materialized_size().write_len();
                    if new_length != old_length {
                        return Err(code_invariant_error(format!(
                            "Trying to patch the value that changed size during materialization: {:?}: {:?} into {:?}. \nValues {:?} into {:?}.", o.key(), old_length, new_length, o.get(), new_write,
                        )));
                    }

                    o.insert(AbstractResourceWriteOp::Write(new_write));
                },
            }
        }
        Ok(())
    }

    /// The events are set to the input events.
    pub(crate) fn set_events(&mut self, patched_events: impl Iterator<Item = ContractEvent>) {
        self.events = patched_events
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
        self,
        resolver: &impl AggregatorV1Resolver,
    ) -> anyhow::Result<Self, VMStatus> {
        let Self {
            resource_write_set,
            module_write_set,
            mut aggregator_v1_write_set,
            aggregator_v1_delta_set,
            delayed_field_change_set,
            events,
        } = self;

        let into_write =
            |(state_key, delta): (StateKey, DeltaOp)| -> anyhow::Result<(StateKey, WriteOp), VMStatus> {
                // Materialization is needed when committing a transaction, so
                // we need precise mode to compute the true value of an
                // aggregator.
                let write = resolver.try_convert_aggregator_v1_delta_into_write_op(&state_key, &delta)?;
                Ok((state_key, write))
            };

        let materialized_aggregator_delta_set = aggregator_v1_delta_set
            .into_iter()
            .map(into_write)
            .collect::<anyhow::Result<BTreeMap<StateKey, WriteOp>, VMStatus>>()?;
        aggregator_v1_write_set.extend(materialized_aggregator_delta_set);

        Ok(Self {
            resource_write_set,
            module_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set: BTreeMap::new(),
            delayed_field_change_set,
            events,
        })
    }

    fn squash_additional_aggregator_v1_changes(
        aggregator_v1_write_set: &mut BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: &mut BTreeMap<StateKey, DeltaOp>,
        additional_aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
        additional_aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
    ) -> anyhow::Result<(), VMStatus> {
        use WriteOp::*;

        // First, squash deltas.
        for (state_key, additional_delta_op) in additional_aggregator_v1_delta_set {
            if let Some(write_op) = aggregator_v1_write_set.get_mut(&state_key) {
                // In this case, delta follows a write op.
                match write_op {
                    Creation(data)
                    | Modification(data)
                    | CreationWithMetadata { data, .. }
                    | ModificationWithMetadata { data, .. } => {
                        // Apply delta on top of creation or modification.
                        // TODO[agg_v1](cleanup): This will not be needed anymore once aggregator
                        // change sets carry non-serialized information.
                        let base: u128 = bcs::from_bytes(data)
                            .expect("Deserializing into an aggregator value always succeeds");
                        let value = additional_delta_op
                            .apply_to(base)
                            .map_err(PartialVMError::from)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                        *data = serialize(&value).into();
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
                match aggregator_v1_delta_set.entry(state_key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming delta
                        // to the existing delta, ensuring the strict ordering.
                        entry
                            .into_mut()
                            .merge_with_next_delta(additional_delta_op)
                            .map_err(PartialVMError::from)
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

    fn squash_additional_delayed_field_changes(
        change_set: &mut BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        additional_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    ) -> anyhow::Result<(), VMStatus> {
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
            change_set.insert(
                id,
                merged_change
                    .map_err(PartialVMError::from)
                    .map_err(|e| e.finish(Location::Undefined).into_vm_status())?,
            );
        }
        Ok(())
    }

    fn squash_additional_module_writes(
        write_set: &mut BTreeMap<StateKey, WriteOp>,
        additional_write_set: BTreeMap<StateKey, WriteOp>,
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

    fn squash_additional_resource_write_ops<
        K: Hash + Eq + PartialEq + Ord + Clone + std::fmt::Debug,
    >(
        write_set: &mut BTreeMap<K, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        additional_write_set: BTreeMap<K, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    ) -> Result<(), PanicError> {
        for (key, additional_entry) in additional_write_set.into_iter() {
            match write_set.entry(key.clone()) {
                Occupied(mut entry) => {
                    // Squash entry and addtional entries if type layouts match
                    let (additional_write_op, additional_type_layout) = additional_entry;
                    let (write_op, type_layout) = entry.get_mut();
                    if *type_layout != additional_type_layout {
                        return Err(code_invariant_error(format!(
                            "Cannot squash two writes with different type layouts.
                            key: {:?}, type_layout: {:?}, additional_type_layout: {:?}",
                            key, type_layout, additional_type_layout
                        )));
                    }
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

    fn squash_additional_resource_writes(
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
                    let (to_delete, to_overwite) = match (entry.get_mut(), &additional_entry) {
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
                            if layout != additional_layout {
                                return Err(code_invariant_error(format!(
                                    "Cannot squash two writes with different type layouts.
                                    key: {:?}, type_layout: {:?}, additional_type_layout: {:?}",
                                    key, layout, additional_layout
                                )));
                            }
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
                        )
                        | (
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
                            // newer read should've read the original write and contain all info from it, but could have additional delayed field writes, that change the size.
                            *materialized_size = Some(*additional_materialized_size);
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

                    if to_overwite {
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
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<(), VMStatus> {
        let Self {
            resource_write_set: additional_resource_write_set,
            module_write_set: additional_module_write_set,
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
        )
        .map_err(|e| {
            VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg(format!("Error while squashing two write ops: {:?}.", e)),
            )
        })?;
        Self::squash_additional_module_writes(
            &mut self.module_write_set,
            additional_module_write_set,
        )?;
        Self::squash_additional_delayed_field_changes(
            &mut self.delayed_field_change_set,
            additional_delayed_field_change_set,
        )?;
        self.events.extend(additional_events);

        checker.check_change_set(self)
    }

    pub fn has_creation(&self) -> bool {
        self.write_set_size_iter()
            .any(|(_key, op_size)| matches!(op_size, WriteOpSize::Creation { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::utils::{mock_tag_0, mock_tag_1, mock_tag_2, raw_metadata};
    use bytes::Bytes;
    use claims::{assert_err, assert_ok, assert_some_eq};
    use test_case::test_case;

    const CREATION: u8 = 0;
    const MODIFICATION: u8 = 1;
    const DELETION: u8 = 2;

    pub(crate) fn write_op_with_metadata(type_idx: u8, v: u128) -> WriteOp {
        match type_idx {
            CREATION => WriteOp::CreationWithMetadata {
                data: vec![].into(),
                metadata: raw_metadata(v as u64),
            },
            MODIFICATION => WriteOp::ModificationWithMetadata {
                data: vec![].into(),
                metadata: raw_metadata(v as u64),
            },
            DELETION => WriteOp::DeletionWithMetadata {
                metadata: raw_metadata(v as u64),
            },
            _ => unreachable!("Wrong type index for test"),
        }
    }

    fn group_write(
        metadata_op: WriteOp,
        inner_ops: Vec<(StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>))>,
        group_size: u64,
    ) -> AbstractResourceWriteOp {
        AbstractResourceWriteOp::WriteResourceGroup(GroupWrite::new(
            metadata_op,
            inner_ops,
            group_size,
        ))
    }

    fn extract_group_op(write_op: &AbstractResourceWriteOp) -> &GroupWrite {
        if let AbstractResourceWriteOp::WriteResourceGroup(write_op) = write_op {
            write_op
        } else {
            panic!("Expected WriteResourceGroup, got {:?}", write_op)
        }
    }

    macro_rules! assert_group_write_size {
        ($op:expr, $s:expr, $exp:expr) => {{
            let group_write = GroupWrite::new($op, vec![], $s);
            assert_eq!(group_write.maybe_group_op_size(), $exp);
        }};
    }

    #[test]
    fn test_group_write_size() {
        // Deletions should lead to size 0.
        assert_group_write_size!(WriteOp::Deletion, 0, None);
        assert_group_write_size!(
            WriteOp::DeletionWithMetadata {
                metadata: raw_metadata(10)
            },
            0,
            None
        );

        let sizes = [20, 100, 45279432, 5];
        assert_group_write_size!(WriteOp::Creation(Bytes::new()), sizes[0], Some(sizes[0]));
        assert_group_write_size!(
            WriteOp::CreationWithMetadata {
                data: Bytes::new(),
                metadata: raw_metadata(20)
            },
            sizes[1],
            Some(sizes[1])
        );
        assert_group_write_size!(
            WriteOp::Modification(Bytes::new()),
            sizes[2],
            Some(sizes[2])
        );
        assert_group_write_size!(
            WriteOp::ModificationWithMetadata {
                data: Bytes::new(),
                metadata: raw_metadata(30)
            },
            sizes[3],
            Some(sizes[3])
        );
    }

    #[test]
    fn test_squash_groups_one_empty() {
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let mut base_update = BTreeMap::new();
        base_update.insert(
            key_1.clone(),
            group_write(write_op_with_metadata(CREATION, 100), vec![], 0),
        );
        let mut additional_update = BTreeMap::new();
        additional_update.insert(
            key_2.clone(),
            group_write(write_op_with_metadata(CREATION, 200), vec![], 0),
        );

        assert_ok!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));

        assert_eq!(base_update.len(), 2);
        assert_some_eq!(
            extract_group_op(base_update.get(&key_1).unwrap())
                .metadata_op
                .metadata(),
            &raw_metadata(100)
        );
        assert_some_eq!(
            extract_group_op(base_update.get(&key_2).unwrap())
                .metadata_op
                .metadata(),
            &raw_metadata(200)
        );
    }

    #[test_case(0, 1)] // create, modify
    #[test_case(1, 1)] // modify, modify
    #[test_case(1, 2)] // modify, delete
    #[test_case(2, 0)] // delete, create
    fn test_squash_groups_mergeable_metadata(base_type_idx: u8, additional_type_idx: u8) {
        let key = StateKey::raw(vec![0]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        base_update.insert(
            key.clone(),
            group_write(write_op_with_metadata(base_type_idx, 100), vec![], 0),
        );
        additional_update.insert(
            key.clone(),
            group_write(write_op_with_metadata(additional_type_idx, 200), vec![], 0),
        );

        assert_ok!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));

        assert_eq!(base_update.len(), 1);
        assert_some_eq!(
            extract_group_op(base_update.get(&key).unwrap())
                .metadata_op
                .metadata(),
            // take the original metadata
            &raw_metadata(100)
        );
    }

    #[test_case(0, 0)] // create, create
    #[test_case(1, 0)] // modify, create
    #[test_case(2, 1)] // delete, modify
    #[test_case(2, 2)] // delete, delete
    fn test_squash_groups_error(base_type_idx: u8, additional_type_idx: u8) {
        let key = StateKey::raw(vec![0]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        base_update.insert(
            key.clone(),
            group_write(write_op_with_metadata(base_type_idx, 100), vec![], 0),
        );
        additional_update.insert(
            key.clone(),
            group_write(write_op_with_metadata(additional_type_idx, 200), vec![], 0),
        );

        assert_err!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));
    }

    #[test]
    fn test_squash_groups_noop() {
        let key = StateKey::raw(vec![0]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        base_update.insert(
            key.clone(),
            group_write(
                write_op_with_metadata(CREATION, 100), // create
                vec![],
                0,
            ),
        );
        additional_update.insert(
            key.clone(),
            group_write(
                write_op_with_metadata(DELETION, 200), // delete
                vec![],
                0,
            ),
        );

        assert_ok!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));
        assert!(base_update.is_empty(), "Must become a no-op");
    }

    #[test]
    fn test_inner_ops() {
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        // TODO[agg_v2](test): Harcoding type layout to None. Test with layout = Some(..)
        base_update.insert(
            key_1.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 100),
                vec![
                    (mock_tag_0(), (WriteOp::Creation(vec![100].into()), None)),
                    (mock_tag_2(), (WriteOp::Modification(vec![2].into()), None)),
                ],
                0,
            ),
        );
        additional_update.insert(
            key_1.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 200),
                vec![
                    (mock_tag_0(), (WriteOp::Modification(vec![0].into()), None)),
                    (mock_tag_1(), (WriteOp::Modification(vec![1].into()), None)),
                ],
                0,
            ),
        );

        base_update.insert(
            key_2.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 100),
                vec![
                    (mock_tag_0(), (WriteOp::Deletion, None)),
                    (mock_tag_1(), (WriteOp::Modification(vec![2].into()), None)),
                    (mock_tag_2(), (WriteOp::Creation(vec![2].into()), None)),
                ],
                0,
            ),
        );
        additional_update.insert(
            key_2.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 200),
                vec![
                    (mock_tag_0(), (WriteOp::Creation(vec![0].into()), None)),
                    (mock_tag_1(), (WriteOp::Deletion, None)),
                    (mock_tag_2(), (WriteOp::Deletion, None)),
                ],
                0,
            ),
        );

        assert_ok!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));
        assert_eq!(base_update.len(), 2);
        let inner_ops_1 = &extract_group_op(base_update.get(&key_1).unwrap()).inner_ops;
        assert_eq!(inner_ops_1.len(), 3);
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_0()),
            &(WriteOp::Creation(vec![0].into()), None)
        );
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_1()),
            &(WriteOp::Modification(vec![1].into()), None)
        );
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_2()),
            &(WriteOp::Modification(vec![2].into()), None)
        );
        let inner_ops_2 = &extract_group_op(base_update.get(&key_2).unwrap()).inner_ops;
        assert_eq!(inner_ops_2.len(), 2);
        assert_some_eq!(
            inner_ops_2.get(&mock_tag_0()),
            &(WriteOp::Modification(vec![0].into()), None)
        );
        assert_some_eq!(inner_ops_2.get(&mock_tag_1()), &(WriteOp::Deletion, None));

        let additional_update = BTreeMap::from([(
            key_2.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 200),
                vec![(mock_tag_1(), (WriteOp::Deletion, None))],
                0,
            ),
        )]);
        assert_err!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));
    }
}

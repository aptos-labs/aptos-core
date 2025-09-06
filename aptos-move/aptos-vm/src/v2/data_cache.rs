// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implementation of data cache for resource and resource groups used by Aptos VM.

use crate::{
    data_cache::get_resource_group_member_from_metadata,
    move_vm_ext::{resource_state_key, AptosMoveResolver},
};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    vm::versioning::{VersionController, VersionedSlot},
    write_set::{TransactionWrite, WriteOp},
};
use aptos_vm_types::{
    abstract_write_op::{
        AbstractResourceWriteOp, GroupWrite, InPlaceDelayedFieldChangeOp,
        ResourceGroupInPlaceDelayedFieldChangeOp,
    },
    change_set::WriteOpInfo,
    resolver::ResourceGroupSize,
    resource_group_adapter::{
        decrement_size_for_remove_tag, group_tagged_resource_size, increment_size_for_add_tag,
    },
    storage::{change_set_configs::ChangeSetSizeTracker, space_pricing::ChargeAndRefund},
};
use bytes::Bytes;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::Op,
    gas_algebra::NumBytes,
    language_storage::{StructTag, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_runtime::{
    data_cache::{MoveVmDataCache, NativeContextMoveVmDataCache},
    module_traversal::TraversalContext,
    native_functions::DependencyGasMeterWrapper,
    FunctionValueExtensionAdapter, LayoutConverter, Loader,
};
use move_vm_types::{
    delayed_values::delayed_field_id::DelayedFieldID,
    gas::DependencyGasMeter,
    loaded_data::runtime_types::Type,
    resolver::resource_size,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::GlobalValue,
};
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet, HashSet},
    sync::Arc,
};

/// Represents different kinds of resources that can be stored in data cache.
enum ResourceKind {
    /// Simple resource.
    Resource {
        /// Key to this resource into global storage.
        state_key: StateKey,
        /// State value metadata associated with this slot. [None] if slot did not exist.
        previous_state_value_metadata: Option<StateValueMetadata>,
        /// Previous size of the value in the slot. Zero if slot did not exist.
        previous_size: u64,
    },
    /// Member of a resource group.
    ResourceGroupMember {
        /// Key for the group, corresponding to the group entries in [TransactionDataCache]. Entries
        /// store all information associated with the group.
        group_key: StateKey,
    },
}

/// Every accessed resource, resource group member or resource group can be materialized into the
/// side effects to the global storage. It is possible that access has no side effects (value was
/// read only), ir ut can be that access mutated the value and thus the new value needs to be
/// written on-chain.
#[derive(Debug, Clone)]
struct MaterializedGlobalValue {
    /// This value was materialized into a write (modification to the on-chain state).
    op: AbstractResourceWriteOp,
    prev_size: u64,
}

/// An entry into data cache corresponding to a resource or resource group member.
struct ResourceEntry {
    /// Specifies the kind of the resource (resource, or a group member).
    kind: ResourceKind,
    /// Tag corresponding to this resource / group member.
    struct_tag: StructTag,
    /// If true, this resource / group member contains delayed fields.
    contains_delayed_fields: bool,
    /// Layout of the resource / group member. Used for (de)serialization.
    layout: Arc<MoveTypeLayout>,
    /// Actual slot containing the value. The slot is versioned, in order to support saving the
    /// current state or undoing certain changes.
    gv: VersionedSlot<GlobalValue>,
    /// Materialization corresponding to the global value (also versioned).
    materialization: VersionedSlot<MaterializedGlobalValue>,
}

/// An entry recorded for the resource group access.
#[derive(Clone)]
struct ResourceGroupEntry {
    /// Metadata associated with the group.
    previous_group_state_value_metadata: Option<StateValueMetadata>,
    /// Previous size of the group.
    previous_group_size: ResourceGroupSize,
}

/// Cache for accessed resources and groups.
pub(crate) struct TransactionDataCache {
    /// Controls versioning of the data cache, keeping track of the right version.
    vc: VersionController,
    /// Cache storing all resources or resource group members.
    resource_cache: BTreeMap<AccountAddress, BTreeMap<Type, ResourceEntry>>,
    /// Cache storing information for every accessed resource group. Unlike resource cache, it is
    /// not versioned because it contains only group state before the transaction.
    group_cache: BTreeMap<StateKey, ResourceGroupEntry>,
    materialized_groups: Option<BTreeMap<StateKey, AbstractResourceWriteOp>>,
}

impl ResourceEntry {
    fn materialize(
        &mut self,
        data_view: &impl AptosMoveResolver,
        loader: &impl Loader,
        new_slot_metadata: &Option<StateValueMetadata>,
        delayed_field_ids: &HashSet<DelayedFieldID>,
        current_version: u32,
        group_cache: &BTreeMap<StateKey, ResourceGroupEntry>,
        materialized_groups: &mut BTreeMap<
            StateKey,
            (
                ResourceGroupSize,
                Option<StateValueMetadata>,
                ResourceGroupSize,
                BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
            ),
        >,
        read_groups_with_delayed_fields: &mut BTreeSet<StateKey>,
        inherit_metadata: bool,
    ) -> PartialVMResult<()> {
        let (gv, gv_version, gv_incarnation, old_materialization) = match self
            .gv
            .needs_derived_recomputation(&mut self.materialization, current_version)
        {
            Some(result) => result,
            None => return Ok(()), // No materialization needed
        };

        match gv.effect() {
            None if !self.contains_delayed_fields => return Ok(()),
            // This is a read, but it contains delayed fields, so it might have been
            // updated.
            None => {
                match &self.kind {
                    ResourceKind::ResourceGroupMember { group_key } => {
                        read_groups_with_delayed_fields.insert(group_key.clone());
                    },
                    ResourceKind::Resource { state_key, .. } => {
                        let metadata_and_size_if_needs_exchange = data_view
                            .as_executor_view()
                            .get_read_needing_exchange(state_key, delayed_field_ids)?;
                        match metadata_and_size_if_needs_exchange {
                            // No delayed fields have been modified, so this is a read.
                            None => (),
                            // Some delayed fields have been modified, so this is a delayed
                            // write: need to materialize.
                            Some((metadata, materialized_size)) => {
                                // TODO: fail fast if metadata or size are different.
                                let change = InPlaceDelayedFieldChangeOp {
                                    layout: self.layout.clone(),
                                    materialized_size,
                                    metadata,
                                };
                                let materialization = MaterializedGlobalValue {
                                    op: AbstractResourceWriteOp::InPlaceDelayedFieldChange(change),
                                    prev_size: materialized_size,
                                };
                                self.materialization.set(
                                    materialization,
                                    gv_version,
                                    gv_incarnation,
                                )?;
                            },
                        }
                    },
                }
            },
            Some(op) => {
                let op = op.and_then(|value| {
                    let function_extension = FunctionValueExtensionAdapter {
                        module_storage: loader.unmetered_module_storage(),
                    };
                    let max_value_nest_depth = function_extension.max_value_nest_depth();

                    let mut serializer = ValueSerDeContext::new(max_value_nest_depth)
                        .with_func_args_deserialization(&function_extension);
                    if self.contains_delayed_fields {
                        serializer = serializer.with_delayed_fields_serde();
                    }

                    serializer
                        .serialize(value, &self.layout)?
                        .ok_or_else(|| {
                            // Note: legacy serialization used a different error code.
                            PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)
                        })
                        .map(Bytes::from)
                })?;

                let maybe_layout = self.contains_delayed_fields.then(|| self.layout.clone());

                match &self.kind {
                    ResourceKind::Resource {
                        previous_state_value_metadata,
                        previous_size,
                        ..
                    } => {
                        let meta = match old_materialization {
                            Some(old_materialization) if inherit_metadata => {
                                Some(old_materialization.op.metadata().clone())
                            },
                            Some(_) | None => previous_state_value_metadata.clone(),
                        };
                        let write_op = build_write_op(op, Some(meta), new_slot_metadata, false)?;
                        let materialization = MaterializedGlobalValue {
                            op: AbstractResourceWriteOp::from_resource_write_with_maybe_layout(
                                write_op,
                                maybe_layout,
                            ),
                            prev_size: *previous_size,
                        };
                        self.materialization
                            .set(materialization, gv_version, gv_incarnation)?;
                    },
                    ResourceKind::ResourceGroupMember { group_key } => {
                        // For resource groups, member writes do not store metadata (they are
                        // converted into legacy write ops).
                        let write_op = build_write_op(op, None, new_slot_metadata, false)?;
                        if !materialized_groups.contains_key(group_key) {
                            let entry = group_cache
                                .get(group_key)
                                .expect("Group must exist for every member");

                            materialized_groups.insert(
                                group_key.clone(),
                                (
                                    entry.previous_group_size,
                                    entry.previous_group_state_value_metadata.clone(),
                                    entry.previous_group_size,
                                    BTreeMap::new(),
                                ),
                            );
                        }

                        let (_, _, post_group_size, group_members) = materialized_groups
                            .get_mut(group_key)
                            .expect("Materialized entry is initialized");
                        if !write_op.is_creation() {
                            let old_tagged_value_size =
                                data_view.resource_size_in_group(group_key, &self.struct_tag)?;
                            let old_size = group_tagged_resource_size(
                                &self.struct_tag,
                                old_tagged_value_size,
                            )?;
                            decrement_size_for_remove_tag(post_group_size, old_size)?;
                        }
                        if !write_op.is_deletion() {
                            let bytes_len =
                                write_op.write_op_size().write_len().unwrap_or(0) as usize;
                            let new_size = group_tagged_resource_size(&self.struct_tag, bytes_len)?;
                            increment_size_for_add_tag(post_group_size, new_size)?;
                        }

                        // Record this as a group write, otherwise ignore materialization.
                        group_members.insert(self.struct_tag.clone(), (write_op, maybe_layout));
                    },
                }
            },
        }
        Ok(())
    }
}

impl TransactionDataCache {
    pub(crate) fn empty() -> Self {
        Self {
            vc: VersionController::new(),
            resource_cache: BTreeMap::new(),
            group_cache: BTreeMap::new(),
            materialized_groups: None,
        }
    }

    pub(crate) fn save(&mut self) {
        self.vc.save();
        self.materialized_groups = None;
    }

    pub(crate) fn undo(&mut self) {
        self.vc.undo();
        self.materialized_groups = None;
    }

    pub(crate) fn materialize(
        &mut self,
        data_view: &impl AptosMoveResolver,
        loader: &impl Loader,
        new_slot_metadata: &Option<StateValueMetadata>,
        delayed_field_ids: &HashSet<DelayedFieldID>,
        inherit_metadata: bool,
    ) -> PartialVMResult<()> {
        let mut read_groups_with_delayed_fields = BTreeSet::new();
        let mut materialized_groups = BTreeMap::new();

        let current_version = self.vc.current_version();
        for (_, account_data_cache) in self.resource_cache.iter_mut() {
            for (_, entry) in account_data_cache.iter_mut() {
                entry.materialize(
                    data_view,
                    loader,
                    new_slot_metadata,
                    delayed_field_ids,
                    current_version,
                    &self.group_cache,
                    &mut materialized_groups,
                    &mut read_groups_with_delayed_fields,
                    inherit_metadata,
                )?;
            }
        }

        let mut materialized_groups: BTreeMap<_, _> = materialized_groups
            .into_iter()
            .map(
                |(key, (pre_group_size, pre_state_value_metadata, post_group_size, members))| {
                    // If we wrote to this group, we do not need to process the reads.
                    read_groups_with_delayed_fields.remove(&key);

                    let metadata_op = if post_group_size.get() == 0 {
                        Op::Delete
                    } else if pre_group_size.get() == 0 {
                        Op::New(Bytes::new())
                    } else {
                        Op::Modify(Bytes::new())
                    };

                    let metadata_to_use = if inherit_metadata {
                        if let Some(op) =  self.materialized_groups.as_ref().and_then(|m | m.get(&key)) {
                            Some(op.metadata().clone())
                        } else {
                            pre_state_value_metadata
                        }
                    } else {
                        pre_state_value_metadata
                    };

                    let metadata_op = build_write_op(
                        metadata_op,
                        Some(metadata_to_use),
                        new_slot_metadata,
                        inherit_metadata,
                    )?;

                    let write_op = AbstractResourceWriteOp::WriteResourceGroup(GroupWrite::new(
                        metadata_op,
                        members,
                        post_group_size,
                        pre_group_size.get(),
                    ));
                    Ok((key, write_op))
                },
            )
            .collect::<PartialVMResult<_>>()?;

        // For all remaining groups that were not written to, we need to check if they had delayed
        // updates.
        for key in read_groups_with_delayed_fields {
            if let Some((metadata, materialized_size)) = data_view
                .as_executor_view()
                .get_group_read_needing_exchange(&key, delayed_field_ids)?
            {
                let change = ResourceGroupInPlaceDelayedFieldChangeOp {
                    // TODO: we may want to check against the cached metadata and size?
                    materialized_size,
                    metadata,
                };
                materialized_groups.insert(
                    key,
                    AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(change),
                );
            }
        }

        self.materialized_groups = Some(materialized_groups);

        Ok(())
    }

    pub(crate) fn charge_write_ops(
        &mut self,
        change_set_size_tracker: &mut ChangeSetSizeTracker,
        gas_meter: &mut impl AptosGasMeter,
    ) -> VMResult<()> {
        // TODO: materialize here
        let current_version = self.vc.current_version();

        for (_, account_data_cache) in self.resource_cache.iter_mut() {
            for (_, entry) in account_data_cache.iter_mut() {
                if let Some(MaterializedGlobalValue { op, prev_size }) = entry
                    .materialization
                    .latest_mut_sync_for_read(current_version)
                {
                    let state_key = match &entry.kind {
                        ResourceKind::Resource { state_key, .. } => state_key,
                        ResourceKind::ResourceGroupMember { .. } => {
                            return Err(PartialVMError::new_invariant_violation(
                                "Group members are not processed!",
                            )
                            .finish(Location::Undefined));
                        },
                    };

                    if let Some(pricing) = change_set_size_tracker.disk_pricing {
                        let ChargeAndRefund { charge, refund } = pricing.charge_refund_write_op(
                            change_set_size_tracker.txn_gas_params.unwrap(),
                            WriteOpInfo {
                                key: state_key,
                                op_size: op.materialized_size(),
                                prev_size: *prev_size,
                                metadata_mut: op.metadata_mut(),
                            },
                        );
                        change_set_size_tracker.write_fee += charge;
                        change_set_size_tracker.total_refund += refund;
                    }

                    change_set_size_tracker.record_write_op(state_key, op.materialized_size())?;
                    gas_meter.charge_io_gas_for_write(state_key, &op.materialized_size())?;
                }
            }
        }

        for (state_key, write_op) in self
            .materialized_groups
            .as_mut()
            .ok_or_else(|| {
                PartialVMError::new_invariant_violation("Must be materialized!")
                    .finish(Location::Undefined)
            })?
            .iter_mut()
        {
            if let Some(pricing) = change_set_size_tracker.disk_pricing {
                let prev_size = match write_op {
                    AbstractResourceWriteOp::WriteResourceGroup(group) => group.prev_group_size(),
                    AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(change) => {
                        change.materialized_size
                    },
                    _ => unreachable!(),
                };
                let ChargeAndRefund { charge, refund } = pricing.charge_refund_write_op(
                    change_set_size_tracker.txn_gas_params.unwrap(),
                    WriteOpInfo {
                        key: state_key,
                        op_size: write_op.materialized_size(),
                        prev_size,
                        metadata_mut: write_op.metadata_mut(),
                    },
                );
                change_set_size_tracker.write_fee += charge;
                change_set_size_tracker.total_refund += refund;
            }

            change_set_size_tracker.record_write_op(state_key, write_op.materialized_size())?;
            gas_meter.charge_io_gas_for_write(state_key, &write_op.materialized_size())?;
        }

        Ok(())
    }

    pub(crate) fn take_writes(&mut self) -> VMResult<BTreeMap<StateKey, AbstractResourceWriteOp>> {
        // TODO: materialize here

        let mut resource_change_set = self.materialized_groups.take().ok_or_else(|| {
            PartialVMError::new_invariant_violation("Must be materialized!")
                .finish(Location::Undefined)
        })?;

        let current_version = self.vc.current_version();
        for (_, account_data_cache) in self.resource_cache.iter_mut() {
            for (_, entry) in account_data_cache.iter_mut() {
                let maybe_write = entry.materialization.take_latest(current_version);
                if let Some(MaterializedGlobalValue { op, .. }) = maybe_write {
                    let state_key = match &entry.kind {
                        ResourceKind::Resource { state_key, .. } => state_key,
                        ResourceKind::ResourceGroupMember { .. } => {
                            return Err(PartialVMError::new_invariant_violation(
                                "Group members are not processed!",
                            )
                            .finish(Location::Undefined));
                        },
                    };

                    resource_change_set.insert(state_key.clone(), op);
                }
            }
        }

        Ok(resource_change_set)
    }
}

/// Adapter to implement [MoveVmDataCache] to pass to the VM to resolve resources or resource
/// group members.
pub(crate) struct TransactionDataCacheAdapter<'a, DataView, CodeLoader> {
    /// Data cache containing all loaded resources.
    data_cache: &'a mut TransactionDataCache,
    /// Global storage for data.
    data_view: &'a DataView,
    /// Code loader (needed to extract metadata to check if resource is a group member.
    loader: &'a CodeLoader,
}

impl<'a, DataView, CodeLoader> TransactionDataCacheAdapter<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    /// Returns the new adapter for the data cache.
    pub fn new(
        data_cache: &'a mut TransactionDataCache,
        data_view: &'a DataView,
        loader: &'a CodeLoader,
    ) -> Self {
        Self {
            data_cache,
            data_view,
            loader,
        }
    }
}

impl<'a, DataView, CodeLoader> NativeContextMoveVmDataCache
    for TransactionDataCacheAdapter<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    fn native_load_resource_check_exists(
        &mut self,
        gas_meter: &mut dyn DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(bool, Option<NumBytes>)> {
        let mut gas_meter = DependencyGasMeterWrapper { gas_meter };
        let (gv, bytes_loaded) = self.load_resource(&mut gas_meter, traversal_context, addr, ty)?;
        let exists = gv.exists()?;
        Ok((exists, bytes_loaded))
    }
}

impl<'a, DataView, CodeLoader> MoveVmDataCache
    for TransactionDataCacheAdapter<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    fn load_resource(
        &mut self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&GlobalValue, Option<NumBytes>)> {
        let bytes_loaded = self.initialize_entries(gas_meter, traversal_context, addr, ty)?;

        let current_version = self.data_cache.vc.current_version();
        let slot = self.get_existing_resource_slot(addr, ty);
        let gv = slot
            .latest(current_version)
            .expect("Slot must be initialized");

        Ok((gv, bytes_loaded))
    }

    fn load_resource_mut(
        &mut self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)> {
        let bytes_loaded = self.initialize_entries(gas_meter, traversal_context, addr, ty)?;

        let current_version = self.data_cache.vc.current_version();
        let slot = self.get_existing_resource_slot(addr, ty);
        let gv = slot
            .latest_mut(current_version)?
            .expect("Slot must be initialized");

        Ok((gv, bytes_loaded))
    }
}

// Private interfaces.
impl<'a, DataView, CodeLoader> TransactionDataCacheAdapter<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    /// Inserts the resource entry into the resource data cache.
    fn insert_resource_entry(
        &mut self,
        addr: &AccountAddress,
        ty: &Type,
        kind: ResourceKind,
        struct_tag: StructTag,
        contains_delayed_fields: bool,
        layout: Arc<MoveTypeLayout>,
        gv: GlobalValue,
    ) -> PartialVMResult<()> {
        let current_version = self.data_cache.vc.current_version();
        match self
            .data_cache
            .resource_cache
            .entry(*addr)
            .or_default()
            .entry(ty.clone())
        {
            Entry::Vacant(entry) => {
                entry.insert(ResourceEntry {
                    kind,
                    struct_tag,
                    contains_delayed_fields,
                    layout,
                    gv: VersionedSlot::new(gv, current_version),
                    materialization: VersionedSlot::empty(),
                });
            },
            Entry::Occupied(entry) => {
                entry.into_mut().gv.set(gv, current_version, 0)?;
            },
        }
        Ok(())
    }

    /// Returns the reference to the slot where resource is stored. Panics if the resource slot has
    /// not been initialized.
    fn get_existing_resource_slot(
        &mut self,
        addr: &AccountAddress,
        ty: &Type,
    ) -> &mut VersionedSlot<GlobalValue> {
        &mut self
            .data_cache
            .resource_cache
            .get_mut(addr)
            .expect("Entry must exist at address")
            .get_mut(ty)
            .expect("Entry must exist for this type")
            .gv
    }

    /// If the provided tag is the member of a resource group, returns the tag of the parent group.
    /// Otherwise, returns [None].
    fn get_resource_group_tag(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        potential_member_tag: &StructTag,
    ) -> PartialVMResult<Option<StructTag>> {
        // TODO: we can cache Aptos metadata and access it directly.
        let metadata = self.loader.load_module_metadata(
            gas_meter,
            traversal_context,
            &potential_member_tag.module_id(),
        )?;
        Ok(get_resource_group_member_from_metadata(
            potential_member_tag,
            &metadata,
        ))
    }

    /// Returns the tag and the layout for the type. Also, returns a boolean indicating if the
    /// layout must contain delayed fields.
    fn get_resource_tag_and_layout(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<(StructTag, bool, Arc<MoveTypeLayout>)> {
        let struct_tag = match self.loader.runtime_environment().ty_to_ty_tag(ty)? {
            TypeTag::Struct(struct_tag) => *struct_tag,
            _ => {
                // Since every resource is a struct, the tag must be also a struct tag.
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR));
            },
        };

        // TODO: cache converter in session, make sure to not share with init module?
        let (layout, contains_delayed_fields) = LayoutConverter::new(self.loader)
            .type_to_type_layout_with_delayed_fields(gas_meter, traversal_context, ty)?
            .unpack();
        Ok((struct_tag, contains_delayed_fields, Arc::new(layout)))
    }

    /// Builds a new slot for the resource / group member entry, deserializing the provided bytes.
    fn build_slot(
        &self,
        addr: &AccountAddress,
        struct_tag: &StructTag,
        layout: &MoveTypeLayout,
        maybe_bytes: Option<&Bytes>,
    ) -> PartialVMResult<GlobalValue> {
        Ok(match maybe_bytes {
            None => GlobalValue::none(),
            Some(bytes) => {
                let function_value_extension = FunctionValueExtensionAdapter {
                    module_storage: self.loader.unmetered_module_storage(),
                };
                let max_value_nest_depth = function_value_extension.max_value_nest_depth();
                let val = ValueSerDeContext::new(max_value_nest_depth)
                    .with_func_args_deserialization(&function_value_extension)
                    .with_delayed_fields_serde()
                    .deserialize(bytes, layout)
                    .ok_or_else(|| {
                        let msg = format!(
                            "Failed to deserialize value for {} at {}",
                            struct_tag.to_canonical_string(),
                            addr.to_hex_literal()
                        );
                        PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_RESOURCE)
                            .with_message(msg)
                    })?;
                GlobalValue::cached(val)?
            },
        })
    }

    /// Initializes the entries in resource and group caches. If the resource or the resource group
    /// member have existed in cache before - a no-op. Returns the number of loaded bytes:
    ///   1. [None] - if entry existed before, otherwise
    ///   2. If resource, its size.
    ///   3. If resource member, its size plus the size of the group if it is the first access to
    ///      it (otherwise member size only).
    fn initialize_entries(
        &mut self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<Option<NumBytes>> {
        let current_version = self.data_cache.vc.current_version();

        if let Some(entry) = self
            .data_cache
            .resource_cache
            .get_mut(addr)
            .and_then(|account_cache| account_cache.get_mut(ty))
        {
            if entry.gv.latest(current_version).is_some() {
                return Ok(None);
            }
            // We need to initialize otherwise.
        }

        let (struct_tag, contains_delayed_fields, layout) =
            self.get_resource_tag_and_layout(gas_meter, traversal_context, ty)?;
        let layout_when_contains_delayed_fields = contains_delayed_fields.then(|| layout.as_ref());

        if let Some(group_tag) =
            self.get_resource_group_tag(gas_meter, traversal_context, &struct_tag)?
        {
            let group_key = StateKey::resource_group(addr, &group_tag);
            let bytes = self
                .data_view
                .as_resource_group_view()
                .get_resource_from_group(
                    &group_key,
                    &struct_tag,
                    layout_when_contains_delayed_fields,
                )?;
            let gv = self.build_slot(addr, &struct_tag, &layout, bytes.as_ref())?;

            let group_size_if_first_access = if self.data_cache.group_cache.contains_key(&group_key)
            {
                0
            } else {
                let previous_group_size = self
                    .data_view
                    .as_resource_group_view()
                    .resource_group_size(&group_key)?;
                let previous_group_state_value_metadata = self
                    .data_view
                    .as_executor_view()
                    .get_resource_state_value_metadata(&group_key)?;

                self.data_cache
                    .group_cache
                    .insert(group_key.clone(), ResourceGroupEntry {
                        previous_group_state_value_metadata,
                        previous_group_size,
                    });
                previous_group_size.get()
            };

            let kind = ResourceKind::ResourceGroupMember { group_key };
            self.insert_resource_entry(
                addr,
                ty,
                kind,
                struct_tag,
                contains_delayed_fields,
                layout,
                gv,
            )?;

            let total_size = resource_size(&bytes) as u64 + group_size_if_first_access;
            Ok(Some(NumBytes::new(total_size)))
        } else {
            let state_key = resource_state_key(addr, &struct_tag)?;

            let bytes = self
                .data_view
                .as_executor_view()
                .get_resource_bytes(&state_key, layout_when_contains_delayed_fields)?;
            let gv = self.build_slot(addr, &struct_tag, &layout, bytes.as_ref())?;

            let previous_state_value_metadata = self
                .data_view
                .as_executor_view()
                .get_resource_state_value_metadata(&state_key)?;
            let previous_size = resource_size(&bytes) as u64;
            let kind = ResourceKind::Resource {
                state_key,
                previous_state_value_metadata,
                previous_size,
            };

            self.insert_resource_entry(
                addr,
                ty,
                kind,
                struct_tag,
                contains_delayed_fields,
                layout,
                gv,
            )?;

            Ok(Some(NumBytes::new(previous_size)))
        }
    }
}

fn build_write_op(
    op: Op<Bytes>,
    state_value_metadata: Option<Option<StateValueMetadata>>,
    new_slot_metadata: &Option<StateValueMetadata>,
    inherit_metadata: bool,
) -> PartialVMResult<WriteOp> {
    Ok(match op {
        Op::New(bytes) => {
            let state_value_metadata = match state_value_metadata {
                Some(state_value_metadata) => state_value_metadata,
                None => {
                    // group member!
                    return Ok(WriteOp::legacy_creation(bytes));
                },
            };

            if inherit_metadata {
                match state_value_metadata {
                    None => WriteOp::legacy_creation(bytes),
                    // TODO: double check this for groups? should it be legacy all the time?
                    Some(metadata) => WriteOp::creation(bytes, metadata),
                }
            } else {
                if state_value_metadata.is_some() {
                    return Err(PartialVMError::new(
                        StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                    ));
                }

                match new_slot_metadata {
                    None => WriteOp::legacy_creation(bytes),
                    // TODO: double check this for groups? should it be legacy all the time?
                    Some(metadata) => WriteOp::creation(bytes, metadata.clone()),
                }
            }
        },
        Op::Modify(bytes) => {
            if let Some(state_value_metadata) = state_value_metadata {
                let state_value_metadata = state_value_metadata.ok_or_else(|| {
                    PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                })?;
                WriteOp::modification(bytes, state_value_metadata)
            } else {
                // group member
                WriteOp::legacy_modification(bytes)
            }
        },
        Op::Delete => {
            if let Some(state_value_metadata) = state_value_metadata {
                let state_value_metadata = state_value_metadata.ok_or_else(|| {
                    PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                })?;
                WriteOp::deletion(state_value_metadata)
            } else {
                // group member
                WriteOp::legacy_deletion()
            }
        },
    })
}

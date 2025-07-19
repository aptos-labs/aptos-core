// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::get_resource_group_member_from_metadata,
    move_vm_ext::{resource_state_key, AptosMoveResolver},
};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    vm::versioning::{VersionController, VersionedSlot},
    write_set::WriteOp,
};
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage,
    resolver::ResourceGroupSize,
    resource_group_adapter::{
        decrement_size_for_remove_tag, group_tagged_resource_size, increment_size_for_add_tag,
    },
    storage::change_set_configs::ChangeSetSizeTracker,
};
use bytes::Bytes;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::Op,
    gas_algebra::NumBytes,
    language_storage::{StructTag, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_runtime::{
    data_cache::MoveVmDataCache, AsFunctionValueExtension, LayoutConverter, StorageLayoutConverter,
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::GlobalValue,
};
use std::{
    collections::{btree_map, BTreeMap},
    sync::Arc,
};

enum ResourceKind {
    Resource {
        #[allow(dead_code)]
        state_key: StateKey,
        state_value_metadata: Option<StateValueMetadata>,
    },
    ResourceGroupMember {
        group_tag: StructTag,
        group_state_key: StateKey,
    },
}

struct DataCacheEntry {
    kind: ResourceKind,
    struct_tag: StructTag,
    contains_delayed_fields: bool,
    layout: Arc<MoveTypeLayout>,
    slot: VersionedSlot<GlobalValue, WriteOp>,
}

impl DataCacheEntry {
    fn build_for_version(
        data_view: &impl AptosMoveResolver,
        code_view: &impl AptosModuleStorage,
        current_version: u32,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(Self, NumBytes)> {
        let struct_tag = match code_view.runtime_environment().ty_to_ty_tag(ty)? {
            TypeTag::Struct(struct_tag) => *struct_tag,
            _ => {
                // Since every resource is a struct, the tag must be also a struct tag.
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR));
            },
        };

        let (layout, contains_delayed_fields) = StorageLayoutConverter::new(code_view)
            .type_to_type_layout_with_identifier_mappings(ty)?;

        // TODO:
        //   1. Cache resource group metadata and fetch it directly.
        //   2. Use executor view with group view, so callers can use adapter.
        let metadata = code_view
            .fetch_existing_module_metadata(&struct_tag.address, struct_tag.module.as_ident_str())
            .map_err(|err| err.to_partial())?;

        let kind = if let Some(group_tag) =
            get_resource_group_member_from_metadata(&struct_tag, &metadata)
        {
            let group_state_key = StateKey::resource_group(addr, &group_tag);
            ResourceKind::ResourceGroupMember {
                group_tag,
                group_state_key,
            }
        } else {
            let state_key = resource_state_key(addr, &struct_tag)?;
            let state_value_metadata = data_view
                .as_executor_view()
                .get_resource_state_value_metadata(&state_key)?;
            ResourceKind::Resource {
                state_key,
                state_value_metadata,
            }
        };

        let (bytes, size) = data_view.get_resource_bytes_with_metadata_and_layout(
            addr,
            &struct_tag,
            &metadata,
            contains_delayed_fields.then_some(&layout),
        )?;

        let gv = match bytes {
            None => GlobalValue::none(),
            Some(blob) => {
                let function_value_extension = code_view.as_function_value_extension();
                let max_value_nest_depth = function_value_extension.max_value_nest_depth();

                let val = ValueSerDeContext::new(max_value_nest_depth)
                    .with_func_args_deserialization(&function_value_extension)
                    .with_delayed_fields_serde()
                    .deserialize(&blob, &layout)
                    .ok_or_else(|| {
                        let msg = format!(
                            "Failed to deserialize resource {} at {}!",
                            struct_tag.to_canonical_string(),
                            addr
                        );
                        PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_RESOURCE)
                            .with_message(msg)
                    })?;
                GlobalValue::cached(val)?
            },
        };
        let entry = Self {
            kind,
            struct_tag,
            contains_delayed_fields,
            layout: Arc::new(layout),
            slot: VersionedSlot::new(gv, current_version),
        };
        Ok((entry, NumBytes::new(size as u64)))
    }
}

struct PendingResourceGroupWrite {
    pre_state_value_metadata: Option<StateValueMetadata>,
    pre_group_size: ResourceGroupSize,
    post_group_size: ResourceGroupSize,
}

impl PendingResourceGroupWrite {
    fn materialize(
        self,
        new_slot_metadata: &Option<StateValueMetadata>,
    ) -> PartialVMResult<MaterializedResourceGroupWrite> {
        let Self {
            pre_state_value_metadata,
            pre_group_size,
            post_group_size,
        } = self;

        let metadata_op = if post_group_size.get() == 0 {
            Op::Delete
        } else if pre_group_size.get() == 0 {
            Op::New(Bytes::new())
        } else {
            Op::Modify(Bytes::new())
        };
        let metadata_op = build_write_op(
            metadata_op,
            Some(pre_state_value_metadata),
            new_slot_metadata,
        )?;
        Ok(MaterializedResourceGroupWrite {
            metadata_op,
            pre_group_size,
            post_group_size,
        })
    }
}

struct MaterializedResourceGroupWrite {
    #[allow(dead_code)]
    metadata_op: WriteOp,
    #[allow(dead_code)]
    pre_group_size: ResourceGroupSize,
    #[allow(dead_code)]
    post_group_size: ResourceGroupSize,
}

pub(crate) struct TransactionDataCache {
    vc: VersionController,
    data_cache: BTreeMap<AccountAddress, BTreeMap<Type, DataCacheEntry>>,
    materialized_groups: BTreeMap<StructTag, MaterializedResourceGroupWrite>,
}

impl TransactionDataCache {
    pub(crate) fn empty() -> Self {
        Self {
            vc: VersionController::new(),
            data_cache: BTreeMap::new(),
            materialized_groups: BTreeMap::new(),
        }
    }

    pub(crate) fn save(&mut self) {}

    pub(crate) fn undo(&mut self) {}

    // Q: should I collect size directly?
    pub(crate) fn materialize(
        &mut self,
        data_view: &impl AptosMoveResolver,
        code_view: &impl AptosModuleStorage,
        new_slot_metadata: &Option<StateValueMetadata>,
    ) -> PartialVMResult<()> {
        // todo: piggyback counters

        let function_extension = code_view.as_function_value_extension();
        let max_value_nest_depth = function_extension.max_value_nest_depth();

        let mut new_materialized_group_sizes = BTreeMap::new();

        let current_version = self.vc.current_version();
        for (_, account_data_cache) in self.data_cache.iter_mut() {
            for (_, entry) in account_data_cache.iter_mut() {
                entry
                    .slot
                    .materialize(current_version, |gv| -> PartialVMResult<_> {
                        let op = match gv.effect() {
                            Some(op) => op,
                            None => {
                                // TODO:
                                //   If this read was used for aggregators, i.e. requires exchange, we need
                                //   to make sure we charge gas for it later.
                                // let state_value_metadata = entry
                                //     .state_value_metadata
                                //     .as_ref()
                                //     .ok_or_else(|| {
                                //         PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                                //     })?;
                                // *write = LazyWriteOp::MaterializedDelayedWrite(state_value_metadata.clone());

                                // TODO:
                                //   If this is a group member: ...

                                unimplemented!()
                            },
                        };

                        let serialize_value = |value| {
                            let mut serializer = ValueSerDeContext::new(max_value_nest_depth)
                                .with_func_args_deserialization(&function_extension);
                            if entry.contains_delayed_fields {
                                serializer = serializer.with_delayed_fields_serde();
                            }

                            serializer
                                .serialize(value, &entry.layout)?
                                .ok_or_else(|| {
                                    // New error code.
                                    PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)
                                })
                                .map(Bytes::from)
                        };

                        let is_creation = matches!(op, Op::New(_));
                        let is_deletion = matches!(op, Op::Delete);

                        let serialized_op = op.and_then(serialize_value)?;
                        let state_value_metadata = match &entry.kind {
                            ResourceKind::Resource {
                                state_value_metadata,
                                ..
                            } => Some(state_value_metadata.clone()),
                            ResourceKind::ResourceGroupMember { .. } => None,
                        };

                        let write_op =
                            build_write_op(serialized_op, state_value_metadata, new_slot_metadata)?;
                        let resource_size = write_op.bytes_size();

                        // We have materialized writes for the group, remove it to invalidate - it needs to
                        // be recomputed now.
                        if let ResourceKind::ResourceGroupMember {
                            group_tag,
                            group_state_key,
                        } = &entry.kind
                        {
                            self.materialized_groups.remove(group_tag);

                            let pending_write =
                                match new_materialized_group_sizes.entry(group_tag.clone()) {
                                    btree_map::Entry::Occupied(entry) => entry.into_mut(),
                                    btree_map::Entry::Vacant(entry) => {
                                        let pre_group_size = data_view
                                            .as_resource_group_view()
                                            .resource_group_size(group_state_key)?;
                                        let pre_state_value_metadata = data_view
                                            .as_executor_view()
                                            .get_resource_state_value_metadata(group_state_key)?;

                                        entry.insert(PendingResourceGroupWrite {
                                            pre_group_size,
                                            pre_state_value_metadata,
                                            post_group_size: pre_group_size,
                                        })
                                    },
                                };

                            if !is_creation {
                                // TODO: cache this on first access
                                let old_tagged_value_size = data_view
                                    .resource_size_in_group(group_state_key, &entry.struct_tag)?;
                                let old_size = group_tagged_resource_size(
                                    &entry.struct_tag,
                                    old_tagged_value_size,
                                )?;
                                decrement_size_for_remove_tag(
                                    &mut pending_write.post_group_size,
                                    old_size,
                                )?;
                            }
                            if !is_deletion {
                                let new_size =
                                    group_tagged_resource_size(&entry.struct_tag, resource_size)?;
                                increment_size_for_add_tag(
                                    &mut pending_write.post_group_size,
                                    new_size,
                                )?;
                            }
                        }

                        Ok(write_op)
                    })?;
            }
        }
        for (group_tag, pending_write) in new_materialized_group_sizes.into_iter() {
            self.materialized_groups
                .insert(group_tag, pending_write.materialize(new_slot_metadata)?);
        }

        Ok(())
    }

    pub(crate) fn charge_write_ops(
        &mut self,
        _change_set_size_tracker: &mut ChangeSetSizeTracker,
        _gas_meter: &mut impl AptosGasMeter,
    ) -> PartialVMResult<()> {
        unimplemented!()
    }
}

/// Adapter to implement [MoveVmDataCache] to pass to the VM to resolve resources or resource
/// group members.
pub(crate) struct TransactionDataCacheAdapter<'a, DataView, CodeView> {
    /// Data cache containing all loaded resources.
    data_cache: &'a mut TransactionDataCache,
    /// Global storage for data.
    data_view: &'a DataView,
    /// Global storage for code (needed to extract metadata to check if resource is a group member.
    code_view: &'a CodeView,
}

impl<'a, DataView, CodeView> TransactionDataCacheAdapter<'a, DataView, CodeView>
where
    DataView: AptosMoveResolver,
    CodeView: AptosModuleStorage,
{
    pub fn new(
        data_cache: &'a mut TransactionDataCache,
        data_view: &'a DataView,
        code_view: &'a CodeView,
    ) -> Self {
        Self {
            data_cache,
            data_view,
            code_view,
        }
    }
}

impl<'a, DataView, CodeView> MoveVmDataCache for TransactionDataCacheAdapter<'a, DataView, CodeView>
where
    DataView: AptosMoveResolver,
    CodeView: AptosModuleStorage,
{
    fn load_resource(
        &mut self,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&GlobalValue, Option<NumBytes>)> {
        let current_version = self.data_cache.vc.current_version();
        match self
            .data_cache
            .data_cache
            .entry(*addr)
            .or_default()
            .entry(ty.clone())
        {
            btree_map::Entry::Occupied(entry) => {
                let gv = entry.into_mut().slot.latest(current_version);
                Ok((gv, None))
            },
            btree_map::Entry::Vacant(entry) => {
                let (data_cache_entry, size) = DataCacheEntry::build_for_version(
                    self.data_view,
                    self.code_view,
                    current_version,
                    addr,
                    ty,
                )?;
                let gv = entry.insert(data_cache_entry).slot.latest(current_version);
                Ok((gv, Some(size)))
            },
        }
    }

    fn load_resource_mut(
        &mut self,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)> {
        let current_version = self.data_cache.vc.current_version();
        match self
            .data_cache
            .data_cache
            .entry(*addr)
            .or_default()
            .entry(ty.clone())
        {
            btree_map::Entry::Occupied(entry) => {
                let gv = entry.into_mut().slot.latest_mut(current_version)?;
                Ok((gv, None))
            },
            btree_map::Entry::Vacant(entry) => {
                let (data_cache_entry, size) = DataCacheEntry::build_for_version(
                    self.data_view,
                    self.code_view,
                    current_version,
                    addr,
                    ty,
                )?;
                let gv = entry
                    .insert(data_cache_entry)
                    .slot
                    .latest_mut(current_version)?;
                Ok((gv, Some(size)))
            },
        }
    }
}

fn build_write_op(
    op: Op<Bytes>,
    state_value_metadata: Option<Option<StateValueMetadata>>,
    new_slot_metadata: &Option<StateValueMetadata>,
) -> PartialVMResult<WriteOp> {
    Ok(match op {
        Op::New(bytes) => {
            if state_value_metadata.is_none() || state_value_metadata.is_some_and(|s| s.is_some()) {
                return Err(PartialVMError::new(
                    StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                ));
            }

            match new_slot_metadata {
                None => WriteOp::legacy_creation(bytes),
                // TODO: double check this for groups? should it be legacy all the time?
                Some(metadata) => WriteOp::creation(bytes, metadata.clone()),
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

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_traversal::TraversalContext,
    storage::{
        module_storage::FunctionValueExtensionAdapter,
        ty_layout_converter::{
            LayoutConverter, MetredLazyLayoutConverter, UnmeteredLayoutConverter,
        },
    },
    ModuleStorage,
};
use bytes::Bytes;
use move_binary_format::errors::*;
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChanges, ChangeSet, Changes},
    gas_algebra::NumBytes,
    language_storage::{StructTag, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::Type,
    resolver::ResourceResolver,
    value_serde::ValueSerDeContext,
    values::{GlobalValue, Value},
};
use std::collections::btree_map::{BTreeMap, Entry};

pub(crate) struct DataCacheEntry {
    struct_tag: StructTag,
    layout: MoveTypeLayout,
    contains_delayed_fields: bool,
    pub(crate) value: GlobalValue,
}

/// Transaction data cache. Keep updates within a transaction so they can all be published at
/// once when the transaction succeeds.
///
/// It also provides an implementation for the opcodes that refer to storage and gives the
/// proper guarantees of reference lifetime.
///
/// Dirty objects are serialized and returned in make_write_set.
///
/// It is a responsibility of the client to publish changes once the transaction is executed.
///
/// The Move VM takes a `DataStore` in input and this is the default and correct implementation
/// for a data store related to a transaction. Clients should create an instance of this type
/// and pass it to the Move VM.
pub struct TransactionDataCache {
    account_map: BTreeMap<AccountAddress, BTreeMap<Type, DataCacheEntry>>,
}

impl TransactionDataCache {
    /// Create a `TransactionDataCache` with a `RemoteCache` that provides access to data
    /// not updated in the transaction.
    pub fn empty() -> Self {
        TransactionDataCache {
            account_map: BTreeMap::new(),
        }
    }

    /// Make a write set from the updated (dirty, deleted) global resources along with
    /// published modules.
    ///
    /// Gives all proper guarantees on lifetime of global data as well.
    pub fn into_effects(self, module_storage: &dyn ModuleStorage) -> PartialVMResult<ChangeSet> {
        let resource_converter =
            |value: Value, layout: MoveTypeLayout, _: bool| -> PartialVMResult<Bytes> {
                let function_value_extension = FunctionValueExtensionAdapter { module_storage };
                ValueSerDeContext::new()
                    .with_func_args_deserialization(&function_value_extension)
                    .serialize(&value, &layout)?
                    .map(Into::into)
                    .ok_or_else(|| {
                        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                            .with_message(format!("Error when serializing resource {}.", value))
                    })
            };
        self.into_custom_effects(&resource_converter)
    }

    /// Same like `into_effects`, but also allows clients to select the format of
    /// produced effects for resources.
    pub fn into_custom_effects<Resource>(
        self,
        resource_converter: &dyn Fn(Value, MoveTypeLayout, bool) -> PartialVMResult<Resource>,
    ) -> PartialVMResult<Changes<Resource>> {
        let mut change_set = Changes::<Resource>::new();
        for (addr, account_data_cache) in self.account_map.into_iter() {
            let mut resources = BTreeMap::new();
            for entry in account_data_cache.into_values() {
                let DataCacheEntry {
                    struct_tag,
                    layout,
                    contains_delayed_fields,
                    value,
                } = entry;
                if let Some(op) = value.into_effect_with_layout(layout) {
                    resources.insert(
                        struct_tag,
                        op.and_then(|(value, layout)| {
                            resource_converter(value, layout, contains_delayed_fields)
                        })?,
                    );
                }
            }
            if !resources.is_empty() {
                change_set
                    .add_account_changeset(addr, AccountChanges::from_resources(resources))
                    .expect("accounts should be unique");
            }
        }

        Ok(change_set)
    }

    pub(crate) fn is_resource_loaded(&self, addr: &AccountAddress, ty: &Type) -> bool {
        self.account_map
            .get(addr)
            .is_some_and(|account_cache| account_cache.contains_key(ty))
    }

    pub(crate) fn store_loaded_resource(
        &mut self,
        addr: AccountAddress,
        ty: Type,
        data_cache_entry: DataCacheEntry,
    ) -> PartialVMResult<()> {
        debug_assert!(!self.is_resource_loaded(&addr, &ty));

        match self.account_map.entry(addr).or_default().entry(ty.clone()) {
            Entry::Vacant(entry) => entry.insert(data_cache_entry),
            Entry::Occupied(_) => {
                let msg = format!("Entry for {:?} at {} already exists", ty, addr);
                let err = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(msg);
                return Err(err);
            },
        };
        Ok(())
    }

    pub(crate) fn get_resource_if_loaded(
        &mut self,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<&mut GlobalValue> {
        debug_assert!(self.is_resource_loaded(addr, ty));

        if let Some(account_cache) = self.account_map.get_mut(addr) {
            if let Some(entry) = account_cache.get_mut(ty) {
                return Ok(&mut entry.value);
            }
        }

        let msg = format!("Resource for {:?} at {} must exist", ty, addr);
        let err =
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(msg);
        Err(err)
    }

    pub(crate) fn load_resource(
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        resource_resolver: &impl ResourceResolver,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(DataCacheEntry, NumBytes)> {
        let use_lazy_loading = module_storage
            .runtime_environment()
            .vm_config()
            .use_lazy_loading;

        let struct_tag = match module_storage.runtime_environment().ty_to_ty_tag(ty)? {
            TypeTag::Struct(struct_tag) => *struct_tag,
            _ => {
                // All resources must be structs, so this cannot happen.
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR));
            },
        };

        let (layout, contains_delayed_fields) = if use_lazy_loading {
            MetredLazyLayoutConverter::new(gas_meter, traversal_context, module_storage)
                .type_to_type_layout_with_identifier_mappings(ty)?
        } else {
            UnmeteredLayoutConverter::new(module_storage)
                .type_to_type_layout_with_identifier_mappings(ty)?
        };

        let (data, bytes_loaded) = {
            if use_lazy_loading {
                let module_id = traversal_context
                    .referenced_module_ids
                    .alloc(struct_tag.module_id());
                if traversal_context
                    .visit_if_not_special_address(module_id.address(), module_id.name())
                {
                    let size = module_storage
                        .unmetered_get_existing_module_size(module_id.address(), module_id.name())
                        .map_err(|err| err.to_partial())?;
                    gas_meter.charge_dependency(
                        false,
                        module_id.address(),
                        module_id.name(),
                        NumBytes::new(size as u64),
                    )?;
                }
            }

            let metadata = module_storage
                .unmetered_get_existing_module_metadata(&struct_tag.address, &struct_tag.module)
                .map_err(|err| err.to_partial())?;

            // If we need to process delayed fields, we pass the type layout to remote storage for
            // any pre-processing. Remote storage should ensure that all delayed fields are
            // replaced with identifiers if the resource comes from the DB.
            resource_resolver.get_resource_bytes_with_metadata_and_layout(
                addr,
                &struct_tag,
                &metadata,
                contains_delayed_fields.then_some(&layout),
            )?
        };
        let bytes_loaded = NumBytes::new(bytes_loaded as u64);

        let function_value_extension = FunctionValueExtensionAdapter { module_storage };
        let value = match data {
            Some(blob) => {
                let val = ValueSerDeContext::new()
                    .with_func_args_deserialization(&function_value_extension)
                    .with_delayed_fields_serde()
                    .deserialize(&blob, &layout)
                    .ok_or_else(|| {
                        let msg =
                            format!("Failed to deserialize resource {} at {}!", struct_tag, addr);
                        PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_RESOURCE)
                            .with_message(msg)
                    })?;

                GlobalValue::cached(val)?
            },
            None => GlobalValue::none(),
        };

        let entry = DataCacheEntry {
            struct_tag,
            layout,
            contains_delayed_fields,
            value,
        };
        Ok((entry, bytes_loaded))
    }

    pub(crate) fn load_resource_for_natives(
        module_storage: &dyn ModuleStorage,
        resource_resolver: &dyn ResourceResolver,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(DataCacheEntry, NumBytes)> {
        let struct_tag = match module_storage.runtime_environment().ty_to_ty_tag(ty)? {
            TypeTag::Struct(struct_tag) => *struct_tag,
            _ => {
                // All resources must be structs, so this cannot happen.
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR));
            },
        };

        // TODO(lazy-loading): charge gas for layouts
        let (layout, contains_delayed_fields) = UnmeteredLayoutConverter::new(module_storage)
            .type_to_type_layout_with_identifier_mappings(ty)?;

        // TODO(lazy-loading): charge gas for access
        let (data, bytes_loaded) = {
            let metadata = module_storage
                .unmetered_get_existing_module_metadata(&struct_tag.address, &struct_tag.module)
                .map_err(|err| err.to_partial())?;

            // If we need to process delayed fields, we pass the type layout to remote storage for
            // any pre-processing. Remote storage should ensure that all delayed fields are
            // replaced with identifiers if the resource comes from the DB.
            resource_resolver.get_resource_bytes_with_metadata_and_layout(
                addr,
                &struct_tag,
                &metadata,
                contains_delayed_fields.then_some(&layout),
            )?
        };
        let bytes_loaded = NumBytes::new(bytes_loaded as u64);

        let function_value_extension = FunctionValueExtensionAdapter { module_storage };
        let value = match data {
            Some(blob) => {
                let val = ValueSerDeContext::new()
                    .with_func_args_deserialization(&function_value_extension)
                    .with_delayed_fields_serde()
                    .deserialize(&blob, &layout)
                    .ok_or_else(|| {
                        let msg =
                            format!("Failed to deserialize resource {} at {}!", struct_tag, addr);
                        PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_RESOURCE)
                            .with_message(msg)
                    })?;

                GlobalValue::cached(val)?
            },
            None => GlobalValue::none(),
        };

        let entry = DataCacheEntry {
            struct_tag,
            layout,
            contains_delayed_fields,
            value,
        };
        Ok((entry, bytes_loaded))
    }
}

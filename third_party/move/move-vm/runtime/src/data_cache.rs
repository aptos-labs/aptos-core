// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_traversal::TraversalContext,
    storage::{
        module_storage::FunctionValueExtensionAdapter,
        ty_layout_converter::{LayoutConverter, StorageLayoutConverter},
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
    gas::DependencyGasMeter,
    loaded_data::runtime_types::Type,
    resolver::ResourceResolver,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::{GlobalValue, Value},
};
use std::collections::btree_map::{BTreeMap, Entry};

pub trait MoveVmDataCache {
    fn load_resource(
        &mut self,
        module_storage: &dyn ModuleStorage,
        gas_meter: &mut dyn DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&GlobalValue, Option<NumBytes>)> {
        let (gv, loaded_bytes) =
            self.load_resource_mut(module_storage, gas_meter, traversal_context, addr, ty)?;
        Ok((gv, loaded_bytes))
    }

    fn load_resource_mut(
        &mut self,
        module_storage: &dyn ModuleStorage,
        gas_meter: &mut dyn DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)>;
}

/// An entry in the data cache, containing resource's [GlobalValue] as well as additional cached
/// information such as tag, layout, and a flag whether there are any delayed fields inside the
/// resource.
struct DataCacheEntry {
    struct_tag: StructTag,
    layout: MoveTypeLayout,
    contains_delayed_fields: bool,
    value: GlobalValue,
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
pub struct TransactionDataCache<'a, DataView> {
    account_map: BTreeMap<AccountAddress, BTreeMap<Type, DataCacheEntry>>,
    data_view: &'a DataView,
}

impl<'a, DataView> TransactionDataCache<'a, DataView>
where
    DataView: ResourceResolver,
{
    /// Create a `TransactionDataCache` with a `RemoteCache` that provides access to data
    /// not updated in the transaction.
    pub fn empty(data_view: &'a DataView) -> Self {
        Self {
            account_map: BTreeMap::new(),
            data_view,
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
                let max_value_nest_depth = function_value_extension.max_value_nest_depth();
                ValueSerDeContext::new(max_value_nest_depth)
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

    /// Retrieves data from the remote on-chain storage and converts it into a [DataCacheEntry].
    /// Also returns the size of the loaded resource in bytes. This method does not add the entry
    /// to the cache - it is the caller's responsibility to add it there.
    fn create_data_cache_entry(
        &self,
        module_storage: &dyn ModuleStorage,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(DataCacheEntry, NumBytes)> {
        let struct_tag = match module_storage.runtime_environment().ty_to_ty_tag(ty)? {
            TypeTag::Struct(struct_tag) => *struct_tag,
            _ => {
                // Since every resource is a struct, the tag must be also a struct tag.
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR));
            },
        };

        // TODO(Gas): Shall we charge for this?
        let (layout, contains_delayed_fields) = StorageLayoutConverter::new(module_storage)
            .type_to_type_layout_with_identifier_mappings(ty)?;

        let (data, bytes_loaded) = {
            let metadata = module_storage
                .fetch_existing_module_metadata(
                    &struct_tag.address,
                    struct_tag.module.as_ident_str(),
                )
                .map_err(|err| err.to_partial())?;

            // If we need to process delayed fields, we pass type layout to remote storage. Remote
            // storage, in turn ensures that all delayed field values are pre-processed.
            self.data_view.get_resource_bytes_with_metadata_and_layout(
                addr,
                &struct_tag,
                &metadata,
                if contains_delayed_fields {
                    Some(&layout)
                } else {
                    None
                },
            )?
        };

        let function_value_extension = FunctionValueExtensionAdapter { module_storage };
        let value = match data {
            Some(blob) => {
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
            None => GlobalValue::none(),
        };

        let entry = DataCacheEntry {
            struct_tag,
            layout,
            contains_delayed_fields,
            value,
        };
        Ok((entry, NumBytes::new(bytes_loaded as u64)))
    }

    /// Returns true if resource has been inserted into the cache. Otherwise, returns false. The
    /// state of the cache does not chang when calling this function.
    fn contains_resource(&self, addr: &AccountAddress, ty: &Type) -> bool {
        self.account_map
            .get(addr)
            .is_some_and(|account_cache| account_cache.contains_key(ty))
    }

    /// Stores a new entry for loaded resource into the data cache. Returns an error if there is an
    /// entry already for the specified address-type pair.
    fn insert_resource(
        &mut self,
        addr: AccountAddress,
        ty: Type,
        data_cache_entry: DataCacheEntry,
    ) -> PartialVMResult<()> {
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

    /// Returns the resource from the data cache. If resource has not been inserted (i.e., it does
    /// not exist in cache), an error is returned.
    fn get_resource_mut(
        &mut self,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<&mut GlobalValue> {
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
}

impl<'a, DataView> MoveVmDataCache for TransactionDataCache<'a, DataView>
where
    DataView: ResourceResolver,
{
    fn load_resource_mut(
        &mut self,
        module_storage: &dyn ModuleStorage,
        _gas_meter: &mut dyn DependencyGasMeter,
        _traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)> {
        let bytes_loaded = if !self.contains_resource(&addr, ty) {
            let (entry, bytes_loaded) = self.create_data_cache_entry(module_storage, &addr, ty)?;
            self.insert_resource(addr, ty.clone(), entry)?;
            Some(bytes_loaded)
        } else {
            None
        };
        Ok((self.get_resource_mut(&addr, ty)?, bytes_loaded))
    }
}

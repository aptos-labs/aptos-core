// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    native_functions::DependencyGasMeterWrapper,
    storage::{loader::traits::ModuleMetadataLoader, ty_layout_converter::LayoutConverter},
    FunctionValueExtensionAdapter, Loader, ModuleStorage, RuntimeEnvironment,
    StructDefinitionLoader,
};
use bytes::Bytes;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChanges, ChangeSet, Changes},
    gas_algebra::NumBytes,
    language_storage::{StructTag, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    data_cache::{MoveVmDataCache, NativeMoveVmDataCache},
    gas::{DependencyGasMeter, GasMeter},
    loaded_data::runtime_types::Type,
    module_traversal::TraversalContext,
    resolver::ResourceResolver,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::{GlobalValue, Reference, Value, VectorRef},
    views::TypeView,
};
use std::collections::{btree_map::Entry, BTreeMap};

/// Adapter for data cache that also stores references to code and data global storages. In case
/// resource is not yet in data cache, global storage is used to add it there.
pub struct LegacyMoveVmDataCacheAdapter<'a, LoaderImpl> {
    data_cache: &'a mut LegacyMoveVmDataCache,
    resource_resolver: &'a dyn ResourceResolver,
    loader: &'a LoaderImpl,
}

impl<'a, LoaderImpl> LegacyMoveVmDataCacheAdapter<'a, LoaderImpl>
where
    LoaderImpl: Loader,
{
    pub fn new(
        data_cache: &'a mut LegacyMoveVmDataCache,
        resource_resolver: &'a dyn ResourceResolver,
        loader: &'a LoaderImpl,
    ) -> Self {
        Self {
            data_cache,
            resource_resolver,
            loader,
        }
    }

    /// Creates a data cache entry for the specified address-type pair. Charges gas for the number
    /// of bytes loaded.
    fn create_and_charge_data_cache_entry(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<DataCacheEntry> {
        let (entry, bytes_loaded) = LegacyMoveVmDataCache::create_data_cache_entry(
            self.loader,
            &LayoutConverter::new(self.loader),
            gas_meter,
            traversal_context,
            &FunctionValueExtensionAdapter {
                module_storage: self.loader.unmetered_module_storage(),
            },
            self.resource_resolver,
            &addr,
            ty,
        )?;
        gas_meter.charge_load_resource(
            addr,
            TypeWithRuntimeEnvironment {
                ty,
                runtime_environment: self.loader.runtime_environment(),
            },
            entry.value().view(),
            bytes_loaded,
        )?;
        Ok(entry)
    }

    /// Loads a resource from the data store and return the number of bytes read from the storage.
    fn load_resource<'c>(
        &'c mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<&'c mut GlobalValue> {
        if !self.data_cache.contains_resource(&addr, ty) {
            let entry =
                self.create_and_charge_data_cache_entry(gas_meter, traversal_context, addr, ty)?;
            self.data_cache.insert_resource(addr, ty.clone(), entry)?;
        }
        self.data_cache.get_resource_mut(&addr, ty)
    }
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

impl DataCacheEntry {
    pub(crate) fn value(&self) -> &GlobalValue {
        &self.value
    }
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
pub struct LegacyMoveVmDataCache {
    account_map: BTreeMap<AccountAddress, BTreeMap<Type, DataCacheEntry>>,
}

impl LegacyMoveVmDataCache {
    /// Create a `LegacyMoveVmDataCache` with a `RemoteCache` that provides access to data
    /// not updated in the transaction.
    pub fn empty() -> Self {
        LegacyMoveVmDataCache {
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
        metadata_loader: &impl ModuleMetadataLoader,
        layout_converter: &LayoutConverter<impl StructDefinitionLoader>,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        function_value_extension: &impl FunctionValueExtension,
        resource_resolver: &dyn ResourceResolver,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(DataCacheEntry, NumBytes)> {
        let struct_tag = match layout_converter.runtime_environment().ty_to_ty_tag(ty)? {
            TypeTag::Struct(struct_tag) => *struct_tag,
            _ => {
                // Since every resource is a struct, the tag must be also a struct tag.
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR));
            },
        };

        let layout_with_delayed_fields = layout_converter.type_to_type_layout_with_delayed_fields(
            gas_meter,
            traversal_context,
            ty,
        )?;

        let (data, bytes_loaded) = {
            let metadata = metadata_loader.load_module_metadata(
                gas_meter,
                traversal_context,
                &struct_tag.module_id(),
            )?;

            // If we need to process delayed fields, we pass type layout to remote storage. Remote
            // storage, in turn ensures that all delayed field values are pre-processed.
            resource_resolver.get_resource_bytes_with_metadata_and_layout(
                addr,
                &struct_tag,
                &metadata,
                layout_with_delayed_fields.layout_when_contains_delayed_fields(),
            )?
        };

        let (layout, contains_delayed_fields) = layout_with_delayed_fields.unpack();
        let value = match data {
            Some(blob) => {
                let max_value_nest_depth = function_value_extension.max_value_nest_depth();
                let val = ValueSerDeContext::new(max_value_nest_depth)
                    .with_func_args_deserialization(function_value_extension)
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

struct TypeWithRuntimeEnvironment<'a, 'b> {
    ty: &'a Type,
    runtime_environment: &'b RuntimeEnvironment,
}

impl TypeView for TypeWithRuntimeEnvironment<'_, '_> {
    fn to_type_tag(&self) -> TypeTag {
        self.runtime_environment.ty_to_ty_tag(self.ty).unwrap()
    }
}

impl<'a, LoaderImpl> NativeMoveVmDataCache for LegacyMoveVmDataCacheAdapter<'a, LoaderImpl>
where
    LoaderImpl: Loader,
{
    fn native_exists(
        &mut self,
        gas_meter: &mut dyn DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(bool, Option<NumBytes>)> {
        Ok(if !self.data_cache.contains_resource(&addr, ty) {
            let (entry, bytes_loaded) = LegacyMoveVmDataCache::create_data_cache_entry(
                self.loader,
                &LayoutConverter::new(self.loader),
                &mut DependencyGasMeterWrapper { gas_meter },
                traversal_context,
                &FunctionValueExtensionAdapter {
                    module_storage: self.loader.unmetered_module_storage(),
                },
                self.resource_resolver,
                &addr,
                ty,
            )?;
            let exists = entry.value().exists()?;
            self.data_cache.insert_resource(addr, ty.clone(), entry)?;
            (exists, Some(bytes_loaded))
        } else {
            let exists = self.data_cache.get_resource_mut(&addr, ty)?.exists()?;
            (exists, None)
        })
    }
}

impl<'a, LoaderImpl> MoveVmDataCache for LegacyMoveVmDataCacheAdapter<'a, LoaderImpl>
where
    LoaderImpl: Loader,
{
    fn move_to(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        is_generic: bool,
        addr: AccountAddress,
        ty: &Type,
        value: Value,
    ) -> PartialVMResult<()> {
        // TODO: get rif of this clone?
        let runtime_environment = self.loader.runtime_environment().clone();
        let gv = self.load_resource(gas_meter, traversal_context, addr, ty)?;

        match gv.move_to(value) {
            Ok(()) => {
                gas_meter.charge_move_to(
                    is_generic,
                    TypeWithRuntimeEnvironment {
                        ty,
                        runtime_environment: &runtime_environment,
                    },
                    gv.view().unwrap(),
                    true,
                )?;
                Ok(())
            },
            Err((err, resource)) => {
                gas_meter.charge_move_to(
                    is_generic,
                    TypeWithRuntimeEnvironment {
                        ty,
                        runtime_environment: &runtime_environment,
                    },
                    &resource,
                    false,
                )?;
                Err(err.with_message(format!("Failed to move resource into {:?}", addr)))
            },
        }
    }

    fn move_from(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        is_generic: bool,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<Value> {
        let runtime_environment = self.loader.runtime_environment();
        let res = self
            .load_resource(gas_meter, traversal_context, addr, ty)?
            .move_from();
        let v = match res {
            Ok(v) => {
                gas_meter.charge_move_from(
                    is_generic,
                    TypeWithRuntimeEnvironment {
                        ty,
                        runtime_environment,
                    },
                    Some(&v),
                )?;
                v
            },
            Err(err) => {
                gas_meter.charge_move_from(
                    is_generic,
                    TypeWithRuntimeEnvironment {
                        ty,
                        runtime_environment,
                    },
                    None::<&Value>,
                )?;
                return Err(err.with_message(format!("Failed to move resource from {:?}", addr)));
            },
        };
        Ok(v)
    }

    fn exists(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        is_generic: bool,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<bool> {
        let gv = self.load_resource(gas_meter, traversal_context, addr, ty)?;
        let exists = gv.exists()?;
        gas_meter.charge_exists(
            is_generic,
            TypeWithRuntimeEnvironment {
                ty,
                runtime_environment: self.loader.runtime_environment(),
            },
            exists,
        )?;
        Ok(exists)
    }

    fn borrow_global(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        is_generic: bool,
        is_mut: bool,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<Value> {
        let res = self
            .load_resource(gas_meter, traversal_context, addr, ty)?
            .borrow_global();
        gas_meter.charge_borrow_global(
            is_mut,
            is_generic,
            TypeWithRuntimeEnvironment {
                ty,
                runtime_environment: self.loader.runtime_environment(),
            },
            res.is_ok(),
        )?;
        res
    }

    fn copy_on_write(&mut self, _reference: &Reference) -> PartialVMResult<()> {
        // Note: legacy data cache had no support for CoW.
        Ok(())
    }

    fn vector_copy_on_write(&mut self, _reference: &VectorRef) -> PartialVMResult<()> {
        // Note: legacy data cache had no support for CoW.
        Ok(())
    }
}

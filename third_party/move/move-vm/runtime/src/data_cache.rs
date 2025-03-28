// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    storage::module_storage::FunctionValueExtensionAdapter, Loader, ModuleStorage,
    RuntimeEnvironment,
};
use bytes::Bytes;
use move_binary_format::errors::*;
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChanges, ChangeSet, Changes},
    gas_algebra::NumBytes,
    language_storage::TypeTag,
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
use std::collections::btree_map::BTreeMap;

pub struct AccountDataCache {
    // The bool flag in the `data_map` indicates whether the resource contains
    // an aggregator or snapshot.
    data_map: BTreeMap<Type, (MoveTypeLayout, GlobalValue, bool)>,
}

impl AccountDataCache {
    fn new() -> Self {
        Self {
            data_map: BTreeMap::new(),
        }
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
pub struct TransactionDataCache {
    account_map: BTreeMap<AccountAddress, AccountDataCache>,
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
        self.into_custom_effects(&resource_converter, module_storage)
    }

    /// Same like `into_effects`, but also allows clients to select the format of
    /// produced effects for resources.
    pub fn into_custom_effects<Resource>(
        self,
        resource_converter: &dyn Fn(Value, MoveTypeLayout, bool) -> PartialVMResult<Resource>,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<Changes<Resource>> {
        let mut change_set = Changes::<Resource>::new();
        for (addr, account_data_cache) in self.account_map.into_iter() {
            let mut resources = BTreeMap::new();
            for (ty, (layout, gv, has_aggregator_lifting)) in account_data_cache.data_map {
                if let Some(op) = gv.into_effect_with_layout(layout) {
                    let struct_tag = match module_storage.runtime_environment().ty_to_ty_tag(&ty)? {
                        TypeTag::Struct(struct_tag) => *struct_tag,
                        _ => return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
                    };
                    resources.insert(
                        struct_tag,
                        op.and_then(|(value, layout)| {
                            resource_converter(value, layout, has_aggregator_lifting)
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

    fn get_mut_or_insert_with<'a, K, V, F>(map: &'a mut BTreeMap<K, V>, k: &K, gen: F) -> &'a mut V
    where
        F: FnOnce() -> (K, V),
        K: Ord,
    {
        if !map.contains_key(k) {
            let (k, v) = gen();
            map.insert(k, v);
        }
        map.get_mut(k).unwrap()
    }

    pub(crate) fn get_resource_if_loaded(
        &mut self,
        addr: &AccountAddress,
        ty: &Type,
    ) -> Option<&mut GlobalValue> {
        self.account_map
            .get_mut(addr)
            .and_then(|account_cache| account_cache.data_map.get_mut(ty).map(|(_, v, _)| v))
    }

    pub(crate) fn insert_resource(
        &mut self,
        addr: AccountAddress,
        ty: Type,
        value: GlobalValue,
        layout: MoveTypeLayout,
        contains_delayed_fields: bool,
    ) -> PartialVMResult<()> {
        let account_cache = Self::get_mut_or_insert_with(&mut self.account_map, &addr, || {
            (addr, AccountDataCache::new())
        });

        let prev = account_cache
            .data_map
            .insert(ty, (layout, value, contains_delayed_fields));
        if prev.is_some() {
            let msg = format!("Inserting resource at {}, but it already exists", addr);
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(msg),
            );
        }
        Ok(())
    }

    pub(crate) fn load_resource_native(
        _resource_resolver: &dyn ResourceResolver,
        _addr: &AccountAddress,
        _ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool, GlobalValue, NumBytes)> {
        // TODO(lazy): implement native loader to implement this for exists_at
        unimplemented!()
    }

    pub(crate) fn load_resource(
        resource_resolver: &impl ResourceResolver,
        runtime_environment: &RuntimeEnvironment,
        loader: &mut impl Loader,
        gas_meter: &mut impl GasMeter,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool, GlobalValue, NumBytes)> {
        let ty_tag = match runtime_environment.ty_to_ty_tag(ty)? {
            TypeTag::Struct(s_tag) => s_tag,
            _ =>
            // non-struct top-level value; can't happen
            {
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
            },
        };
        let (ty_layout, contains_delayed_fields) =
            loader.load_layout_with_delayed_fields_check(gas_meter, ty)?;

        let (data, bytes_loaded) = {
            let module = loader
                .load_module(gas_meter, &ty_tag.module_id())
                .map_err(|err| err.to_partial())?;

            // If we need to process delayed fields, we pass the type layout to the remote storage.
            // Remote storage, in turn, ensures that all delayed fields are pre-processed if the
            // resource comes from storage.
            resource_resolver.get_resource_bytes_with_metadata_and_layout(
                addr,
                &ty_tag,
                &module.metadata,
                if contains_delayed_fields {
                    Some(&ty_layout)
                } else {
                    None
                },
            )?
        };

        // TODO(lazy): fix value extension
        // let function_value_extension = FunctionValueExtensionAdapter { module_storage };
        let gv = match data {
            Some(blob) => {
                let val = match ValueSerDeContext::new()
                    // .with_func_args_deserialization(&function_value_extension)
                    .with_delayed_fields_serde()
                    .deserialize(&blob, &ty_layout)
                {
                    Some(val) => val,
                    None => {
                        let msg = format!("Failed to deserialize resource {} at {}!", ty_tag, addr);
                        return Err(PartialVMError::new(
                            StatusCode::FAILED_TO_DESERIALIZE_RESOURCE,
                        )
                        .with_message(msg));
                    },
                };

                GlobalValue::cached(val)?
            },
            None => GlobalValue::none(),
        };

        Ok((
            ty_layout,
            contains_delayed_fields,
            gv,
            NumBytes::new(bytes_loaded as u64),
        ))
    }
}

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::loader::{Loader, ModuleStorageAdapter};
use bytes::Bytes;
use move_binary_format::errors::*;
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChanges, ChangeSet, Changes, Op},
    gas_algebra::NumBytes,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    resolver::MoveResolver,
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{GlobalValue, Value},
};
use std::collections::btree_map::BTreeMap;

pub struct AccountDataCache {
    // The bool flag in the `data_map` indicates whether the resource contains
    // an aggregator or snapshot.
    data_map: BTreeMap<Type, (MoveTypeLayout, GlobalValue, bool)>,
    module_map: BTreeMap<Identifier, (Bytes, bool)>,
}

impl AccountDataCache {
    fn new() -> Self {
        Self {
            data_map: BTreeMap::new(),
            module_map: BTreeMap::new(),
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
pub(crate) struct TransactionDataCache<'r> {
    remote: &'r dyn MoveResolver<PartialVMError>,
    account_map: BTreeMap<AccountAddress, AccountDataCache>,
}

impl<'r> TransactionDataCache<'r> {
    /// Create a `TransactionDataCache` with a `RemoteCache` that provides access to data
    /// not updated in the transaction.
    pub(crate) fn new(remote: &'r impl MoveResolver<PartialVMError>) -> Self {
        TransactionDataCache {
            remote,
            account_map: BTreeMap::new(),
        }
    }

    /// Make a write set from the updated (dirty, deleted) global resources along with
    /// published modules.
    ///
    /// Gives all proper guarantees on lifetime of global data as well.
    pub(crate) fn into_effects(self, loader: &Loader) -> PartialVMResult<ChangeSet> {
        let resource_converter =
            |value: Value, layout: MoveTypeLayout, _: bool| -> PartialVMResult<Bytes> {
                value
                    .simple_serialize(&layout)
                    .map(Into::into)
                    .ok_or_else(|| {
                        PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                            .with_message(format!("Error when serializing resource {}.", value))
                    })
            };
        self.into_custom_effects(&resource_converter, loader)
    }

    /// Same like `into_effects`, but also allows clients to select the format of
    /// produced effects for resources.
    pub(crate) fn into_custom_effects<Resource>(
        self,
        resource_converter: &dyn Fn(Value, MoveTypeLayout, bool) -> PartialVMResult<Resource>,
        loader: &Loader,
    ) -> PartialVMResult<Changes<Bytes, Resource>> {
        let mut change_set = Changes::<Bytes, Resource>::new();
        for (addr, account_data_cache) in self.account_map.into_iter() {
            let mut modules = BTreeMap::new();
            for (module_name, (module_blob, is_republishing)) in account_data_cache.module_map {
                let op = if is_republishing {
                    Op::Modify(module_blob)
                } else {
                    Op::New(module_blob)
                };
                modules.insert(module_name, op);
            }

            let mut resources = BTreeMap::new();
            for (ty, (layout, gv, has_aggregator_lifting)) in account_data_cache.data_map {
                if let Some(op) = gv.into_effect_with_layout(layout) {
                    let struct_tag = match loader.type_to_type_tag(&ty)? {
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
            if !modules.is_empty() || !resources.is_empty() {
                change_set
                    .add_account_changeset(
                        addr,
                        AccountChanges::from_modules_resources(modules, resources),
                    )
                    .expect("accounts should be unique");
            }
        }

        Ok(change_set)
    }

    pub(crate) fn num_mutated_accounts(&self, sender: &AccountAddress) -> u64 {
        // The sender's account will always be mutated.
        let mut total_mutated_accounts: u64 = 1;
        for (addr, entry) in self.account_map.iter() {
            if addr != sender && entry.data_map.values().any(|(_, v, _)| v.is_mutated()) {
                total_mutated_accounts += 1;
            }
        }
        total_mutated_accounts
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

    // Retrieves data from the local cache or loads it from the remote cache into the local cache.
    // All operations on the global data are based on this API and they all load the data
    // into the cache.
    pub(crate) fn load_resource(
        &mut self,
        loader: &Loader,
        addr: AccountAddress,
        ty: &Type,
        module_store: &ModuleStorageAdapter,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)> {
        let account_cache = Self::get_mut_or_insert_with(&mut self.account_map, &addr, || {
            (addr, AccountDataCache::new())
        });

        let mut load_res = None;
        if !account_cache.data_map.contains_key(ty) {
            let ty_tag = match loader.type_to_type_tag(ty)? {
                TypeTag::Struct(s_tag) => s_tag,
                _ =>
                // non-struct top-level value; can't happen
                {
                    return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
                },
            };
            // TODO(Gas): Shall we charge for this?
            let (ty_layout, has_aggregator_lifting) =
                loader.type_to_type_layout_with_identifier_mappings(ty, module_store)?;

            let module = module_store.module_at(&ty_tag.module_id());
            let metadata: &[Metadata] = match &module {
                Some(module) => &module.module().metadata,
                None => &[],
            };

            // If we need to process aggregator lifting, we pass type layout to remote.
            // Remote, in turn ensures that all aggregator values are lifted if the resolved
            // resource comes from storage.
            let (data, bytes_loaded) = self.remote.get_resource_bytes_with_metadata_and_layout(
                &addr,
                &ty_tag,
                metadata,
                if has_aggregator_lifting {
                    Some(&ty_layout)
                } else {
                    None
                },
            )?;
            load_res = Some(NumBytes::new(bytes_loaded as u64));

            let gv = match data {
                Some(blob) => {
                    let val = match Value::simple_deserialize(&blob, &ty_layout) {
                        Some(val) => val,
                        None => {
                            let msg =
                                format!("Failed to deserialize resource {} at {}!", ty_tag, addr);
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

            account_cache
                .data_map
                .insert(ty.clone(), (ty_layout, gv, has_aggregator_lifting));
        }

        Ok((
            account_cache
                .data_map
                .get_mut(ty)
                .map(|(_ty_layout, gv, _has_aggregator_lifting)| gv)
                .expect("global value must exist"),
            load_res,
        ))
    }

    pub(crate) fn load_module(&self, module_id: &ModuleId) -> PartialVMResult<Bytes> {
        if let Some(account_cache) = self.account_map.get(module_id.address()) {
            if let Some((blob, _is_republishing)) = account_cache.module_map.get(module_id.name()) {
                return Ok(blob.clone());
            }
        }
        match self.remote.get_module(module_id)? {
            Some(bytes) => Ok(bytes),
            None => Err(
                PartialVMError::new(StatusCode::LINKER_ERROR).with_message(format!(
                    "Linker Error: Cannot find {:?} in data cache",
                    module_id
                )),
            ),
        }
    }

    pub(crate) fn publish_module(
        &mut self,
        module_id: &ModuleId,
        blob: Vec<u8>,
        is_republishing: bool,
    ) -> VMResult<()> {
        let account_cache =
            Self::get_mut_or_insert_with(&mut self.account_map, module_id.address(), || {
                (*module_id.address(), AccountDataCache::new())
            });

        account_cache
            .module_map
            .insert(module_id.name().to_owned(), (blob.into(), is_republishing));

        Ok(())
    }

    pub(crate) fn exists_module(&self, module_id: &ModuleId) -> VMResult<bool> {
        if let Some(account_cache) = self.account_map.get(module_id.address()) {
            if account_cache.module_map.contains_key(module_id.name()) {
                return Ok(true);
            }
        }
        Ok(self
            .remote
            .get_module(module_id)
            .map_err(|e| e.finish(Location::Undefined))?
            .is_some())
    }
}

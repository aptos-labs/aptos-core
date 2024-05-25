// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{Loader, ModuleStorageAdapter},
    logging::expect_no_verification_errors,
};
use bytes::Bytes;
use move_binary_format::{
    deserializer::DeserializerConfig, errors::*, file_format::CompiledScript,
};
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChangeSet, ChangeSet, Op},
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
    value_serde::deserialize_and_allow_delayed_values,
    values::{GlobalValue, Value},
};
use sha3::{Digest, Sha3_256};
use std::{
    collections::btree_map::{self, BTreeMap},
    sync::Arc,
};

struct AccountDataCache<M> {
    // The bool flag in the `resources` indicates whether the resource contains
    // an aggregator or snapshot.
    resources: BTreeMap<Type, (MoveTypeLayout, GlobalValue, bool)>,
    modules: BTreeMap<Identifier, ((M, Bytes), bool)>,
}

impl<M: Clone> AccountDataCache<M> {
    fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            modules: BTreeMap::new(),
        }
    }
}

fn load_module_impl<M: Clone>(
    remote: &dyn MoveResolver<M, PartialVMError>,
    account_map: &BTreeMap<AccountAddress, AccountDataCache<M>>,
    module_id: &ModuleId,
) -> PartialVMResult<(M, usize, [u8; 32])> {
    if let Some(account_cache) = account_map.get(module_id.address()) {
        if let Some(((m, b), _is_republishing)) = account_cache.modules.get(module_id.name()) {
            // FIXME(George): Is it better to cache this information on publish?
            //  In any case, once we move verification to deploy-time (and on storage
            //  load in adapter), this can be removed.
            let mut sha3_256 = Sha3_256::new();
            sha3_256.update(b);
            let hash_value: [u8; 32] = sha3_256.finalize().into();

            return Ok((m.clone(), b.len(), hash_value));
        }
    }
    remote.get_module(module_id)?.ok_or_else(|| {
        PartialVMError::new(StatusCode::LINKER_ERROR)
            .with_message(format!("Linker Error: Module {} doesn't exist", module_id))
    })
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
pub(crate) struct TransactionDataCache<'r, M> {
    remote: &'r dyn MoveResolver<M, PartialVMError>,
    account_map: BTreeMap<AccountAddress, AccountDataCache<M>>,

    deserializer_config: DeserializerConfig,
    compiled_scripts: BTreeMap<[u8; 32], Arc<CompiledScript>>,
}

impl<'r, M: Clone> TransactionDataCache<'r, M> {
    pub(crate) fn empty(
        deserializer_config: DeserializerConfig,
        remote: &'r impl MoveResolver<M, PartialVMError>,
    ) -> Self {
        TransactionDataCache {
            remote,
            account_map: BTreeMap::new(),
            deserializer_config,
            compiled_scripts: BTreeMap::new(),
        }
    }

    /// Make a write set from the updated (dirty, deleted) global resources along with
    /// published modules.
    ///
    /// Gives all proper guarantees on lifetime of global data as well.
    pub(crate) fn into_effects(
        self,
        loader: &Loader,
    ) -> PartialVMResult<ChangeSet<(M, Bytes), Bytes>> {
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
    ) -> PartialVMResult<ChangeSet<(M, Bytes), Resource>> {
        let mut change_set = ChangeSet::<(M, Bytes), Resource>::empty();
        for (addr, account_data_cache) in self.account_map.into_iter() {
            let mut modules = BTreeMap::new();
            for (module_name, (module_blob, is_republishing)) in account_data_cache.modules {
                let op = if is_republishing {
                    Op::Modify(module_blob)
                } else {
                    Op::New(module_blob)
                };
                modules.insert(module_name, op);
            }

            let mut resources = BTreeMap::new();
            for (ty, (layout, gv, has_aggregator_lifting)) in account_data_cache.resources {
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
                    .add_account_change_set(addr, AccountChangeSet::new(modules, resources))
                    .expect("accounts should be unique");
            }
        }

        Ok(change_set)
    }

    pub(crate) fn num_mutated_accounts(&self, sender: &AccountAddress) -> u64 {
        // The sender's account will always be mutated.
        let mut total_mutated_accounts: u64 = 1;
        for (addr, entry) in self.account_map.iter() {
            if addr != sender && entry.resources.values().any(|(_, v, _)| v.is_mutated()) {
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
        if !account_cache.resources.contains_key(ty) {
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
                    let val = match deserialize_and_allow_delayed_values(&blob, &ty_layout) {
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
                .resources
                .insert(ty.clone(), (ty_layout, gv, has_aggregator_lifting));
        }

        Ok((
            account_cache
                .resources
                .get_mut(ty)
                .map(|(_ty_layout, gv, _has_aggregator_lifting)| gv)
                .expect("global value must exist"),
            load_res,
        ))
    }

    pub(crate) fn load_module(&self, module_id: &ModuleId) -> PartialVMResult<M> {
        Ok(load_module_impl(self.remote, &self.account_map, module_id)?.0)
    }

    pub(crate) fn load_compiled_script_to_cache(
        &mut self,
        script_blob: &[u8],
        hash_value: [u8; 32],
    ) -> VMResult<Arc<CompiledScript>> {
        let cache = &mut self.compiled_scripts;
        match cache.entry(hash_value) {
            btree_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            btree_map::Entry::Vacant(entry) => {
                let script = match CompiledScript::deserialize_with_config(
                    script_blob,
                    &self.deserializer_config,
                ) {
                    Ok(script) => script,
                    Err(err) => {
                        let msg = format!("[VM] deserializer for script returned error: {:?}", err);
                        return Err(PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                            .with_message(msg)
                            .finish(Location::Script));
                    },
                };
                Ok(entry.insert(Arc::new(script)).clone())
            },
        }
    }

    pub(crate) fn load_compiled_module_to_cache(
        &mut self,
        module_id: &ModuleId,
        allow_loading_failure: bool,
    ) -> VMResult<(M, usize, [u8; 32])> {
        match load_module_impl(self.remote, &self.account_map, module_id)
            .map_err(|err| err.finish(Location::Undefined))
        {
            Ok(data) => Ok(data),
            Err(err) if allow_loading_failure => Err(err),
            Err(err) => Err(expect_no_verification_errors(err)),
        }
    }

    pub(crate) fn publish_module(
        &mut self,
        module_id: ModuleId,
        module: M,
        blob: Bytes,
        is_republishing: bool,
    ) -> VMResult<()> {
        let account_cache =
            Self::get_mut_or_insert_with(&mut self.account_map, module_id.address(), || {
                (*module_id.address(), AccountDataCache::new())
            });

        account_cache.modules.insert(
            module_id.name().to_owned(),
            ((module, blob), is_republishing),
        );

        Ok(())
    }

    pub(crate) fn exists_module(&self, module_id: &ModuleId) -> VMResult<bool> {
        if let Some(account_cache) = self.account_map.get(module_id.address()) {
            if account_cache.modules.contains_key(module_id.name()) {
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

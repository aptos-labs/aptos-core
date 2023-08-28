// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::{
    aptos_vm_impl::gas_config,
    move_vm_ext::{get_max_binary_format_version, AptosMoveResolver},
};
#[allow(unused_imports)]
use anyhow::Error;
use aptos_aggregator::{
    aggregator_extension::AggregatorID,
    resolver::{AggregatorReadMode, AggregatorResolver},
};
use aptos_framework::natives::state_storage::StateStorageUsageResolver;
use aptos_state_view::{StateView, TStateView};
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::{ConfigStorage, Features, OnChainConfig},
    state_store::{
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
    },
};
use aptos_vm_types::resolver::{
    ResourceGroupResolver, StateValueMetadataResolver, TResourceGroupResolver,
};
use claims::assert_none;
use move_binary_format::{errors::*, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    resolver::{resource_size, ModuleResolver, ResourceResolver},
    vm_status::StatusCode,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
};

pub(crate) fn get_resource_group_from_metadata(
    struct_tag: &StructTag,
    metadata: &[Metadata],
) -> Option<StructTag> {
    let metadata = aptos_framework::get_metadata(metadata)?;
    metadata
        .struct_attributes
        .get(struct_tag.name.as_ident_str().as_str())?
        .iter()
        .find_map(|attr| attr.get_resource_group_member())
}

// Enum for backwards compatibility.
enum ResourceGroupCache {
    // Storage adapter used to cache the whole BTreeMap to resolve reads from the resource groups.
    V0(HashMap<StateKey, BTreeMap<StructTag, Vec<u8>>>),
    // Resource group reads are resolved via dedicated resolver, but HashSet is still needed to
    // distinguish the first access (as it has a different charging behavior).
    V1(HashSet<StateKey>),
}

/// Adapter to convert a `StateView` into a `MoveResolverExt`. Resource group member resources
/// can be optionally resolved via an externally provided resolver.
pub struct StorageAdapter<'a, S, R> {
    state_store: &'a S,
    /// Externally provided resolver for resource groups, currently can come from either (1) block
    /// executor, or (2) respawned session. bool determines whether the resolver should be used
    /// in 'forward' (propagate the output directly) or 'fallback' (on error, default resolve via
    /// state_store) mode - true means 'forward'. For example, (1) needs forwarding while (2)
    /// requires fallback behavior.
    maybe_resource_group_resolver: Option<(&'a R, bool)>,
    accurate_byte_count: bool,
    group_byte_count_as_sum: bool,
    max_binary_format_version: u32,
    resource_group_cache: RefCell<ResourceGroupCache>,
}

impl<'a, S: StateView, R: ResourceGroupResolver> StorageAdapter<'a, S, R> {
    fn init(mut self, features: &Features, gas_feature_version: u64) -> Self {
        if gas_feature_version >= 9 {
            if gas_feature_version >= 12 {
                self.group_byte_count_as_sum = true;
            } else {
                // Versions for 9 to 11 (incl) have behavior based on the serialized size of the
                // BTreeMap of the resource group. We keep the old behavior for replay.
                *self.resource_group_cache.borrow_mut() = ResourceGroupCache::V0(HashMap::new());

                // Provided resolver can handle gas_feature_version >= 12 (as sum), or < 9 (no size),
                // but otherwise we should use the default behavior that can handle any gas setting.
                self.maybe_resource_group_resolver = None;
            }
            self.accurate_byte_count = true;
        }
        self.max_binary_format_version =
            get_max_binary_format_version(features, gas_feature_version);

        self
    }

    pub fn new_with_cached_config(
        state_store: &'a S,
        gas_feature_version: u64,
        features: &Features,
        maybe_resource_group_resolver: Option<(&'a R, bool)>,
    ) -> Self {
        let s = Self {
            state_store,
            maybe_resource_group_resolver,
            accurate_byte_count: false,
            group_byte_count_as_sum: false,
            max_binary_format_version: 0,
            resource_group_cache: RefCell::new(ResourceGroupCache::V1(HashSet::new())),
        };
        s.init(features, gas_feature_version)
    }

    pub fn new(state_store: &'a S) -> Self {
        let s = Self {
            state_store,
            maybe_resource_group_resolver: None,
            accurate_byte_count: false,
            group_byte_count_as_sum: false,
            max_binary_format_version: 0,
            resource_group_cache: RefCell::new(ResourceGroupCache::V1(HashSet::new())),
        };
        let (_, gas_feature_version) = gas_config(&s);
        let features = Features::fetch_config(&s).unwrap_or_default();
        s.init(&features, gas_feature_version)
    }

    pub fn get(&self, access_path: AccessPath) -> PartialVMResult<Option<Vec<u8>>> {
        self.state_store
            .get_state_value_bytes(&StateKey::access_path(access_path))
            .map_err(|_| PartialVMError::new(StatusCode::STORAGE_ERROR))
    }

    fn get_any_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> Result<(Option<Vec<u8>>, usize), VMError> {
        let resource_group = get_resource_group_from_metadata(struct_tag, metadata);
        if let Some(resource_group) = resource_group {
            let key = StateKey::access_path(AccessPath::resource_group_access_path(
                *address,
                resource_group.clone(),
            ));

            let first_access = match &mut *self.resource_group_cache.borrow_mut() {
                ResourceGroupCache::V0(ref mut btree_cache) => {
                    if let Some(group_data) = btree_cache.get_mut(&key) {
                        // This resource group is already V0-cached for this address.
                        // So just return the cached value.
                        let buf = group_data.get(struct_tag).cloned();
                        let buf_size = resource_size(&buf);
                        return Ok((buf, buf_size));
                    } else {
                        true
                    }
                },
                ResourceGroupCache::V1(ref mut accesses) => accesses.insert(key.clone()),
            };

            let (buf, maybe_group_size) = self
                .get_resource_from_group(&key, struct_tag, first_access && self.accurate_byte_count)
                .map_err(|e| {
                    if self.group_byte_count_as_sum {
                        // Message also gated with the byte counting as sum flag / gas version.
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .with_message(format!("{}", e))
                            .finish(Location::Undefined)
                    } else {
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .finish(Location::Undefined)
                    }
                })?;

            let buf_size = resource_size(&buf);
            Ok((buf, buf_size + maybe_group_size.unwrap_or(0)))
        } else {
            let ap =
                AccessPath::resource_access_path(*address, struct_tag.clone()).map_err(|_| {
                    PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Undefined)
                })?;

            let buf = self.get(ap).map_err(|e| e.finish(Location::Undefined))?;
            let buf_size = resource_size(&buf);
            Ok((buf, buf_size))
        }
    }
}

impl<'a, S: StateView, R: ResourceGroupResolver> AptosMoveResolver for StorageAdapter<'a, S, R> {
    fn release_resource_group_cache(
        &self,
    ) -> Option<HashMap<StateKey, BTreeMap<StructTag, Vec<u8>>>> {
        let empty_cache = if matches!(
            &*self.resource_group_cache.borrow(),
            ResourceGroupCache::V0(_)
        ) {
            assert!(
                self.accurate_byte_count && !self.group_byte_count_as_sum,
                "ResourceGroupCache V0 used in wrong setting"
            );
            ResourceGroupCache::V0(HashMap::new())
        } else {
            ResourceGroupCache::V1(HashSet::new())
        };

        // Clears the cache in both cases, and in V0 case returns the cache (replay compatibility)
        if let ResourceGroupCache::V0(btree) = self.resource_group_cache.replace(empty_cache) {
            Some(btree)
        } else {
            // V1 accesses cleared by take(), nothing to return.
            None
        }
    }
}

impl<'a, S: StateView, R: ResourceGroupResolver> TResourceGroupResolver
    for StorageAdapter<'a, S, R>
{
    type Key = StateKey;
    type Tag = StructTag;

    fn get_resource_from_group(
        &self,
        key: &StateKey,
        resource_tag: &StructTag,
        return_group_size: bool,
    ) -> anyhow::Result<(Option<Vec<u8>>, Option<usize>)> {
        // Forward to resource group resolver, if provided.
        if let Some((resolver, forward)) = self.maybe_resource_group_resolver {
            let res = resolver.get_resource_from_group(key, resource_tag, return_group_size);

            if forward || res.is_ok() {
                // Return is forwarding is set, or resolution succeeded (no fallback needed).
                return res;
            }
        }

        let mut v0_group_to_cache = BTreeMap::new();

        // Resolve directly from state store (StateView interface).
        let group_data = self.state_store.get_state_value_bytes(key)?;
        let ret = if let Some(group_data_blob) = group_data {
            let group_data: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(&group_data_blob)
                .map_err(|_| anyhow::Error::msg("Resource group deserialization error"))?;

            let maybe_group_size = return_group_size.then_some({
                assert!(
                    self.accurate_byte_count,
                    "No charge for first access, should not be returning size"
                );

                if self.group_byte_count_as_sum {
                    // Computing the size based on the sizes of the elements in group_data.
                    group_data
                        .iter()
                        .try_fold(0, |len, (tag, res)| {
                            let delta = bcs::serialized_size(tag)? + res.len();
                            Ok(len + delta)
                        })
                        .map_err(|_: Error| {
                            anyhow::Error::msg("Resource group member tag serialization error")
                        })?
                } else {
                    // Computing the size based on the serialized length of group_data.
                    group_data_blob.len()
                }
            });

            let res = group_data.get(resource_tag).cloned();

            v0_group_to_cache = group_data;

            Ok((res, maybe_group_size))
        } else {
            Ok((None, None))
        };

        if let ResourceGroupCache::V0(ref mut cache) = &mut *self.resource_group_cache.borrow_mut()
        {
            assert_none!(cache.insert(key.clone(), v0_group_to_cache));
        }

        ret
    }

    fn resource_exists_within_group(
        &self,
        key: &StateKey,
        resource_tag: &StructTag,
    ) -> anyhow::Result<bool> {
        if let Some((resolver, _)) = self.maybe_resource_group_resolver {
            return resolver.resource_exists_within_group(key, resource_tag);
        }

        // If no resolver is provided, we can simply fallback to get_resource interface.
        self.get_resource_from_group(key, resource_tag, false)
            .map(|(res, _)| res.is_some())
    }
}

impl<'a, S: StateView, R: ResourceGroupResolver> ResourceResolver for StorageAdapter<'a, S, R> {
    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<(Option<Vec<u8>>, usize)> {
        Ok(self.get_any_resource(address, struct_tag, metadata)?)
    }
}

impl<'a, S: StateView, R: ResourceGroupResolver> ModuleResolver for StorageAdapter<'a, S, R> {
    fn get_module_metadata(&self, module_id: &ModuleId) -> Vec<Metadata> {
        let module_bytes = match self.get_module(module_id) {
            Ok(Some(bytes)) => bytes,
            _ => return vec![],
        };
        let module = match CompiledModule::deserialize_with_max_version(
            &module_bytes,
            self.max_binary_format_version,
        ) {
            Ok(module) => module,
            _ => return vec![],
        };
        module.metadata
    }

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Error> {
        // REVIEW: cache this?
        let ap = AccessPath::from(module_id);
        Ok(self.get(ap).map_err(|e| e.finish(Location::Undefined))?)
    }
}

impl<'a, S: StateView, R: ResourceGroupResolver> TableResolver for StorageAdapter<'a, S, R> {
    fn resolve_table_entry(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_state_value_bytes(&StateKey::table_item((*handle).into(), key.to_vec()))
    }
}

impl<'a, S: StateView, R: ResourceGroupResolver> AggregatorResolver for StorageAdapter<'a, S, R> {
    fn resolve_aggregator_value(
        &self,
        id: &AggregatorID,
        _mode: AggregatorReadMode,
    ) -> Result<u128, Error> {
        let AggregatorID { handle, key } = id;
        let state_key = StateKey::table_item(*handle, key.0.to_vec());
        match self.get_state_value_u128(&state_key)? {
            Some(value) => Ok(value),
            None => {
                anyhow::bail!("Could not find the value of the aggregator")
            },
        }
    }

    fn generate_aggregator_id(&self) -> AggregatorID {
        unimplemented!("Aggregator id generation will be implemented for V2 aggregators.")
    }
}

impl<'a, S: StateView, R: ResourceGroupResolver> ConfigStorage for StorageAdapter<'a, S, R> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Vec<u8>> {
        self.get(access_path).ok()?
    }
}

impl<'a, S: StateView, R: ResourceGroupResolver> StateStorageUsageResolver
    for StorageAdapter<'a, S, R>
{
    fn get_state_storage_usage(&self) -> Result<StateStorageUsage, Error> {
        self.state_store.get_usage()
    }
}

pub trait AsMoveResolver<S> {
    fn as_move_resolver(&self) -> StorageAdapter<S, ()>;
}

impl<S: StateView> AsMoveResolver<S> for S {
    fn as_move_resolver(&self) -> StorageAdapter<S, ()> {
        StorageAdapter::new(self)
    }
}

impl<'a, S: StateView, R: ResourceGroupResolver> StateValueMetadataResolver
    for StorageAdapter<'a, S, R>
{
    fn get_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<Option<StateValueMetadata>>> {
        let maybe_state_value = self.state_store.get_state_value(state_key)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }
}

// We need to implement StateView for adapter because:
//   1. When processing write set payload, storage is accessed
//      directly.
//   2. When stacking Storage adapters on top of each other, e.g.
//      in epilogue.
impl<'a, S: StateView, R: ResourceGroupResolver> TStateView for StorageAdapter<'a, S, R> {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        self.state_store.get_state_value(state_key)
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.state_store.get_usage()
    }
}

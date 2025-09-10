// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    types::TxnIndex, versioned_data::VersionedData,
    versioned_delayed_fields::VersionedDelayedFields, versioned_group_data::VersionedGroupData,
};
use aptos_types::{
    executable::ModulePath, vm::modules::AptosModuleExtension, write_set::TransactionWrite,
};
use move_binary_format::{file_format::CompiledScript, CompiledModule};
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::{Module, Script};
use move_vm_types::code::{ModuleCache, ModuleCode, SyncModuleCache, SyncScriptCache};
use serde::Serialize;
use std::{fmt::Debug, hash::Hash, sync::Arc};

mod registered_dependencies;
pub mod types;
pub mod unsync_map;
pub mod versioned_data;
pub mod versioned_delayed_fields;
pub mod versioned_group_data;

#[cfg(test)]
mod unit_tests;

/// Main multi-version data-structure used by threads to read/write during parallel
/// execution.
///
/// Concurrency is managed by DashMap, i.e. when a method accesses a BTreeMap at a
/// given key, it holds exclusive access and doesn't need to explicitly synchronize
/// with other reader/writers.
///
/// TODO(BlockSTMv2): consider handling the baseline retrieval inside MVHashMap, by
/// providing a lambda during construction. This would simplify the caller logic and
/// allow unifying initialization logic e.g. for resource groups that span two
/// different multi-version data-structures (MVData and MVGroupData). It would also
/// allow performing a check on the path once during initialization (to determine
/// if the path is for a resource or a group), and then checking invariants.
pub struct MVHashMap<K, T, V: TransactionWrite, I: Clone> {
    data: VersionedData<K, V>,
    group_data: VersionedGroupData<K, T, V>,
    delayed_fields: VersionedDelayedFields<I>,

    module_cache:
        SyncModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension, Option<TxnIndex>>,
    script_cache: SyncScriptCache<[u8; 32], CompiledScript, Script>,
}

impl<K, T, V, I> MVHashMap<K, T, V, I>
where
    K: ModulePath + Hash + Clone + Eq + Debug,
    T: Hash + Clone + Eq + Debug + Serialize,
    V: TransactionWrite + PartialEq,
    I: Copy + Clone + Eq + Hash + Debug,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> MVHashMap<K, T, V, I> {
        #[allow(deprecated)]
        MVHashMap {
            data: VersionedData::empty(),
            group_data: VersionedGroupData::empty(),
            delayed_fields: VersionedDelayedFields::empty(),

            module_cache: SyncModuleCache::empty(),
            script_cache: SyncScriptCache::empty(),
        }
    }

    pub fn stats(&self) -> BlockStateStats {
        BlockStateStats {
            num_resources: self.data.num_keys(),
            num_resource_groups: self.group_data.num_keys(),
            num_delayed_fields: self.delayed_fields.num_keys(),
            num_modules: self.module_cache.num_modules(),
            base_resources_size: self.data.total_base_value_size(),
            base_delayed_fields_size: self.delayed_fields.total_base_value_size(),
        }
    }

    /// Contains 'simple' versioned data (nothing contained in groups).
    pub fn data(&self) -> &VersionedData<K, V> {
        &self.data
    }

    /// Contains data representing resource groups, or more generically, internally
    /// containing different values mapped to tags of type T.
    pub fn group_data(&self) -> &VersionedGroupData<K, T, V> {
        &self.group_data
    }

    pub fn delayed_fields(&self) -> &VersionedDelayedFields<I> {
        &self.delayed_fields
    }

    /// Returns the module cache. While modules in it are associated with versions, at any point
    /// in time throughout block execution the cache contains 1) modules from pre-block state or,
    /// 2) committed modules.
    pub fn module_cache(
        &self,
    ) -> &SyncModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension, Option<TxnIndex>>
    {
        &self.module_cache
    }

    /// Takes module from module cache and returns an iterator to the taken keys and modules.
    pub fn take_modules_iter(
        &mut self,
    ) -> impl Iterator<
        Item = (
            ModuleId,
            Arc<ModuleCode<CompiledModule, Module, AptosModuleExtension>>,
        ),
    > {
        self.module_cache.take_modules_iter()
    }

    /// Returns the script cache.
    pub fn script_cache(&self) -> &SyncScriptCache<[u8; 32], CompiledScript, Script> {
        &self.script_cache
    }
}

pub struct BlockStateStats {
    pub num_resources: usize,
    pub num_resource_groups: usize,
    pub num_delayed_fields: usize,
    pub num_modules: usize,

    pub base_resources_size: u64,
    pub base_delayed_fields_size: u64,
}

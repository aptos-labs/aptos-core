// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    captured_reads::CacheRead,
    counters::GLOBAL_MODULE_CACHE_MISS_SECONDS,
    view::{LatestView, ViewState},
};
use ambassador::delegate_to_methods;
use aptos_mvhashmap::types::TxnIndex;
#[cfg(test)]
use aptos_types::on_chain_config::CurrentTimeMicroseconds;
use aptos_types::{
    executable::ModulePath,
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::AptosModuleExtension,
};
use aptos_vm_types::module_and_script_storage::module_storage::AptosModuleStorage;
#[cfg(test)]
use fail::fail_point;
use move_binary_format::{
    errors::{Location, PartialVMResult, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
};
use move_vm_runtime::{
    LayoutCache, LayoutCacheEntry, Module, RuntimeEnvironment, Script, StructKey,
    WithRuntimeEnvironment,
};
use move_vm_types::code::{
    ambassador_impl_ScriptCache, Code, ModuleCache, ModuleCode, ModuleCodeBuilder, ScriptCache,
    WithBytes,
};
use std::sync::Arc;

impl<T: Transaction, S: TStateView<Key = T::Key>> WithRuntimeEnvironment for LatestView<'_, T, S> {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.runtime_environment
    }
}

impl<T: Transaction, S: TStateView<Key = T::Key>> ModuleCodeBuilder for LatestView<'_, T, S> {
    type Deserialized = CompiledModule;
    type Extension = AptosModuleExtension;
    type Key = ModuleId;
    type Verified = Module;

    fn build(
        &self,
        key: &Self::Key,
    ) -> VMResult<Option<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        let constructed_key = T::Key::from_address_and_module_name(key.address(), key.name());
        self.get_raw_base_value(&constructed_key)
            .map_err(|err| err.finish(Location::Undefined))?
            .map(|mut state_value| {
                // TODO: remove this once framework on mainnet is using the new option module
                if let Some(bytes) = self
                    .runtime_environment()
                    .get_module_bytes_override(key.address(), key.name())
                {
                    state_value.set_bytes(bytes);
                }
                let extension = Arc::new(AptosModuleExtension::new(state_value));
                let compiled_module = self
                    .runtime_environment()
                    .deserialize_into_compiled_module(extension.bytes())?;
                Ok(ModuleCode::from_deserialized(compiled_module, extension))
            })
            .transpose()
    }
}

impl<T: Transaction, S: TStateView<Key = T::Key>> ModuleCache for LatestView<'_, T, S> {
    type Deserialized = CompiledModule;
    type Extension = AptosModuleExtension;
    type Key = ModuleId;
    type Verified = Module;
    type Version = Option<TxnIndex>;

    fn insert_deserialized_module(
        &self,
        key: Self::Key,
        deserialized_code: Self::Deserialized,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        self.as_module_cache().insert_deserialized_module(
            key,
            deserialized_code,
            extension,
            version,
        )
    }

    fn insert_verified_module(
        &self,
        key: Self::Key,
        verified_code: Self::Verified,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        match &self.latest_view {
            ViewState::Sync(state) => {
                // For parallel execution, if we insert a verified module, we might need to also
                // update module cache in captured reads so that they also store the verified code.
                // If we do not do that, reads to module cache will end up reading "old" code that
                // is stored in captured reads and is not verified.
                let module = state.versioned_map.module_cache().insert_verified_module(
                    key.clone(),
                    verified_code,
                    extension,
                    version,
                )?;
                state
                    .captured_reads
                    .borrow_mut()
                    .capture_per_block_cache_read(key, Some((module.clone(), version)));
                Ok(module)
            },
            ViewState::Unsync(state) => state.unsync_map.module_cache().insert_verified_module(
                key,
                verified_code,
                extension,
                version,
            ),
        }
    }

    fn get_module_or_build_with(
        &self,
        key: &Self::Key,
        builder: &dyn ModuleCodeBuilder<
            Key = Self::Key,
            Deserialized = Self::Deserialized,
            Verified = Self::Verified,
            Extension = Self::Extension,
        >,
    ) -> VMResult<
        Option<(
            Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>,
            Self::Version,
        )>,
    > {
        match &self.latest_view {
            ViewState::Sync(state) => {
                // Check the transaction-level cache with already read modules first.
                if let CacheRead::Hit(read) = state.captured_reads.borrow().get_module_read(key) {
                    return Ok(read);
                }

                // Otherwise, it is a miss. Check global cache.
                if let Some(module) = self.global_module_cache.get(key) {
                    state
                        .captured_reads
                        .borrow_mut()
                        .capture_global_cache_read(key.clone(), module.clone());
                    return Ok(Some((module, Self::Version::default())));
                }

                // If not global cache, check per-block cache.
                let _timer = GLOBAL_MODULE_CACHE_MISS_SECONDS.start_timer();
                let read = state
                    .versioned_map
                    .module_cache()
                    .get_module_or_build_with(key, builder)?;
                state
                    .captured_reads
                    .borrow_mut()
                    .capture_per_block_cache_read(key.clone(), read.clone());
                Ok(read)
            },
            ViewState::Unsync(state) => {
                if let Some(module) = self.global_module_cache.get(key) {
                    state.read_set.borrow_mut().capture_module_read(key.clone());
                    return Ok(Some((module, Self::Version::default())));
                }

                let _timer = GLOBAL_MODULE_CACHE_MISS_SECONDS.start_timer();
                let read = state
                    .unsync_map
                    .module_cache()
                    .get_module_or_build_with(key, builder)?;
                state.read_set.borrow_mut().capture_module_read(key.clone());
                Ok(read)
            },
        }
    }

    fn num_modules(&self) -> usize {
        self.as_module_cache().num_modules()
    }
}

impl<T: Transaction, S: TStateView<Key = T::Key>> AptosModuleStorage for LatestView<'_, T, S> {
    fn unmetered_get_module_state_value_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        let result = self
            .get_module_or_build_with(&id, self)
            .map_err(|err| err.to_partial())?;

        // In order to test the module cache with combinatorial tests, we embed the version
        // information into the state value metadata (execute_transaction has access via
        // AptosModuleStorage trait only).
        #[cfg(test)]
        fail_point!("module_test", |_| {
            Ok(result.clone().map(|(_, version)| {
                let v = version.unwrap_or(u32::MAX) as u64;
                StateValueMetadata::legacy(v, &CurrentTimeMicroseconds { microseconds: v })
            }))
        });

        Ok(result.map(|(module, _)| module.extension().state_value_metadata().clone()))
    }
}

#[delegate_to_methods]
#[delegate(ScriptCache, target_ref = "as_script_cache")]
impl<T: Transaction, S: TStateView<Key = T::Key>> LatestView<'_, T, S> {
    /// Returns the script cache.
    fn as_script_cache(
        &self,
    ) -> &dyn ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script> {
        match &self.latest_view {
            ViewState::Sync(state) => state.versioned_map.script_cache(),
            ViewState::Unsync(state) => state.unsync_map.script_cache(),
        }
    }

    /// Returns the module cache.
    fn as_module_cache(
        &self,
    ) -> &dyn ModuleCache<
        Key = ModuleId,
        Deserialized = CompiledModule,
        Verified = Module,
        Extension = AptosModuleExtension,
        Version = Option<TxnIndex>,
    > {
        match &self.latest_view {
            ViewState::Sync(state) => state.versioned_map.module_cache(),
            ViewState::Unsync(state) => state.unsync_map.module_cache(),
        }
    }
}

impl<T: Transaction, S: TStateView<Key = T::Key>> LayoutCache for LatestView<'_, T, S> {
    fn get_struct_layout(&self, key: &StructKey) -> Option<LayoutCacheEntry> {
        self.global_module_cache.get_struct_layout_entry(key)
    }

    fn store_struct_layout(&self, key: &StructKey, entry: LayoutCacheEntry) -> PartialVMResult<()> {
        self.global_module_cache
            .store_struct_layout_entry(key, entry)?;
        Ok(())
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValueMetadata, StateView},
    vm::modules::{ModuleStorageEntry, ModuleStorageEntryInterface},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::module_and_script_storage::AsAptosCodeStorage;
use arc_swap::ArcSwapOption;
use bytes::Bytes;
use claims::assert_ok;
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::ModuleId,
    metadata::Metadata,
};
use move_vm_runtime::{Module, ModuleStorage, RuntimeEnvironment};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{ops::Deref, sync::Arc};

/// Represents a unique identifier for an [AptosEnvironment] instance based on the features, gas
/// feature version, and other configs.
#[derive(Hash, Eq, PartialEq)]
struct EnvironmentID {
    bytes: Bytes,
}

impl EnvironmentID {
    /// Create a new identifier for the given environment.
    fn new(env: &AptosEnvironment) -> Self {
        // These are sufficient to distinguish different environments.
        let chain_id = env.chain_id();
        let features = env.features();
        let timed_features = env.timed_features();
        let gas_feature_version = env.gas_feature_version();
        let vm_config = env.vm_config();
        let bytes = bcs::to_bytes(&(
            chain_id,
            features,
            timed_features,
            gas_feature_version,
            vm_config,
        ))
        .expect("Should be able to serialize all configs")
        .into();
        Self { bytes }
    }
}

/// A cached environment that can be persisted across blocks. Used by block executor only. Also
/// stores an identifier so that we can check when it changes.
pub struct CachedAptosEnvironment {
    id: EnvironmentID,
    env: AptosEnvironment,
}

impl CachedAptosEnvironment {
    /// Returns the cached environment if it exists and has the same configuration as if it was
    /// created based on the current state, or creates a new one and caches it. Should only be
    /// called at the block boundaries.
    pub fn fetch_with_delayed_field_optimization_enabled(
        state_view: &impl StateView,
    ) -> AptosEnvironment {
        // Create a new environment.
        let env = AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);
        let id = EnvironmentID::new(&env);

        // Lock the cache, and check if the environment is the same.
        let mut cross_block_environment = CROSS_BLOCK_ENVIRONMENT.lock();
        if let Some(cached_env) = cross_block_environment.as_ref() {
            if id == cached_env.id {
                return cached_env.env.clone();
            }
        }

        // It is not, so we have to reset it. Also flush the framework cache because we need to
        // re-load all the modules with new configs.
        *cross_block_environment = Some(CachedAptosEnvironment {
            id,
            env: env.clone(),
        });
        flush_cross_block_framework_cache();
        drop(cross_block_environment);

        env
    }
}

static CROSS_BLOCK_ENVIRONMENT: Lazy<Mutex<Option<CachedAptosEnvironment>>> =
    Lazy::new(|| Mutex::new(None));

/// Initializes the module cache for the framework that can live over multiple blocks. Should be
/// called at block boundaries.
pub fn initialize_cross_block_framework_cache(
    state_view: &impl StateView,
    runtime_environment: &RuntimeEnvironment,
) {
    // We need to check that the framework is really cached. We check if transaction validation
    // module is cached.
    if check_module_exists_in_cross_block_framework_cache(
        &AccountAddress::ONE,
        ident_str!("transaction_validation"),
    ) {
        return;
    }

    let mut cache = hashbrown::HashMap::new();
    let module_storage = state_view.as_aptos_code_storage(runtime_environment);
    let framework = aptos_cached_packages::head_release_bundle();

    // We should not be using the code because it is tied to the release and whatever exists in the
    // state view currently. We only use compiled modules to be able to fetch addresses and module
    // names (since package metadata does not store the address information)
    for (_, module) in framework.code_and_compiled_modules() {
        let module_id = module.self_id();

        let state_key = StateKey::module_id(&module_id);
        let state_value = assert_ok!(state_view.get_state_value(&state_key));
        let module =
            assert_ok!(module_storage.fetch_verified_module(module_id.address(), module_id.name()));

        if let (Some(state_value), Some(module)) = (state_value, module) {
            let entry =
                ModuleStorageEntry::from_state_value_and_verified_module(state_value, module);
            cache.insert(module_id, entry);
        }
    }

    CROSS_BLOCK_FRAMEWORK_CACHE.store(Some(Arc::new(cache)));
}

/// Returns the state value metadata from the cross module framework cache. If the module has not
/// been cached, or the access is not for the framework, [None] is returned.
pub(crate) fn fetch_module_state_value_metadata_from_cross_block_framework_cache(
    address: &AccountAddress,
    module_name: &IdentStr,
) -> Option<StateValueMetadata> {
    let maybe_cache = CROSS_BLOCK_FRAMEWORK_CACHE.load();
    if let Some(cache) = maybe_cache.deref().as_ref() {
        return cache
            .get(&(address, module_name))
            .map(|e| e.state_value_metadata().clone());
    }
    None
}

/// Returns the true if the module exists in the cross module framework cache. If the module has
/// not been cached, or the access is not for the framework, false is returned.
pub(crate) fn check_module_exists_in_cross_block_framework_cache(
    address: &AccountAddress,
    module_name: &IdentStr,
) -> bool {
    let maybe_cache = CROSS_BLOCK_FRAMEWORK_CACHE.load();
    if let Some(cache) = maybe_cache.deref().as_ref() {
        return cache.contains_key(&(address, module_name));
    }
    false
}

/// Returns the module size in bytes from the cross module framework cache. If the module has not
/// been cached, or the access is not for the framework, [None] is returned.
pub(crate) fn fetch_module_size_in_bytes_from_cross_block_framework_cache(
    address: &AccountAddress,
    module_name: &IdentStr,
) -> Option<usize> {
    let maybe_cache = CROSS_BLOCK_FRAMEWORK_CACHE.load();
    if let Some(cache) = maybe_cache.deref().as_ref() {
        return cache
            .get(&(address, module_name))
            .map(|e| e.size_in_bytes());
    }
    None
}

/// Returns the module bytes from the cross module framework cache. If the module has not been
/// cached, or the access is not for the framework, [None] is returned.
pub(crate) fn fetch_module_bytes_from_cross_block_framework_cache(
    address: &AccountAddress,
    module_name: &IdentStr,
) -> Option<Bytes> {
    let maybe_cache = CROSS_BLOCK_FRAMEWORK_CACHE.load();
    if let Some(cache) = maybe_cache.deref().as_ref() {
        return cache
            .get(&(address, module_name))
            .map(|e| e.bytes().clone());
    }
    None
}

/// Returns the metadat from the module from the cross module framework cache. If the module has
/// not been cached, or the access is not for the framework, [None] is returned.
pub(crate) fn fetch_module_metadata_from_cross_block_framework_cache(
    address: &AccountAddress,
    module_name: &IdentStr,
) -> Option<Vec<Metadata>> {
    let maybe_cache = CROSS_BLOCK_FRAMEWORK_CACHE.load();
    if let Some(cache) = maybe_cache.deref().as_ref() {
        return cache
            .get(&(address, module_name))
            .map(|e| e.metadata().to_vec());
    }
    None
}

/// Returns the deserialized module from the cross module framework cache. If the module has not
/// been cached, or the access is not for the framework, [None] is returned.
pub(crate) fn fetch_deserialized_module_from_cross_block_framework_cache(
    address: &AccountAddress,
    module_name: &IdentStr,
) -> Option<Arc<CompiledModule>> {
    let maybe_cache = CROSS_BLOCK_FRAMEWORK_CACHE.load();
    if let Some(cache) = maybe_cache.deref().as_ref() {
        return cache
            .get(&(address, module_name))
            .map(|e| e.as_compiled_module());
    }
    None
}

/// Returns the verified module from the cross module framework cache. If the module has not been
/// cached, or the access is not for the framework, [None] is returned.
pub(crate) fn fetch_verified_module_from_cross_block_framework_cache(
    address: &AccountAddress,
    module_name: &IdentStr,
) -> Option<Arc<Module>> {
    let maybe_cache = CROSS_BLOCK_FRAMEWORK_CACHE.load();
    if let Some(cache) = maybe_cache.deref().as_ref() {
        return cache.get(&(address, module_name)).map(|e| {
            e.try_as_verified_module()
                .expect("Modules stored in framework cache are always verified")
        });
    }
    None
}

/// Flushes the cross-block cache for framework modules. Used when the framework is upgraded.
pub(crate) fn flush_cross_block_framework_cache() {
    CROSS_BLOCK_FRAMEWORK_CACHE.store(None)
}

static CROSS_BLOCK_FRAMEWORK_CACHE: Lazy<
    ArcSwapOption<hashbrown::HashMap<ModuleId, ModuleStorageEntry>>,
> = Lazy::new(|| ArcSwapOption::new(None));

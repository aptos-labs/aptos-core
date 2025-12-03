// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::anyhow;
use aptos_logger::info;
use aptos_types::{
    on_chain_config::{Features, OnChainConfig},
    state_store::{state_key::StateKey, state_value::StateValue, StateView, StateViewId},
    vm::modules::AptosModuleExtension,
};
use aptos_vm_environment::{
    environment::AptosEnvironment, prod_configs::aptos_prod_deserializer_config,
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use move_binary_format::{
    deserializer::DeserializerConfig,
    errors::{Location, PartialVMError, VMResult},
    CompiledModule,
};
use move_bytecode_utils::compiled_module_viewer::CompiledModuleView;
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_runtime::{
    Module, ModuleStorage, NoOpLayoutCache, RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_vm_types::{
    code::{ModuleCache, ModuleCode, ModuleCodeBuilder, UnsyncModuleCache},
    module_storage_error,
};
use std::{cell::RefCell, collections::HashMap, sync::Arc};

pub struct ModuleView<'a, S> {
    module_cache: RefCell<HashMap<ModuleId, Arc<CompiledModule>>>,
    deserializer_config: DeserializerConfig,
    state_view: &'a S,
}

impl<'a, S: StateView> ModuleView<'a, S> {
    pub fn new(state_view: &'a S) -> Self {
        let features = Features::fetch_config(state_view).unwrap_or_default();
        let deserializer_config = aptos_prod_deserializer_config(&features);

        Self {
            module_cache: RefCell::new(HashMap::new()),
            deserializer_config,
            state_view,
        }
    }
}

impl<S: StateView> CompiledModuleView for ModuleView<'_, S> {
    type Item = Arc<CompiledModule>;

    fn view_compiled_module(&self, module_id: &ModuleId) -> anyhow::Result<Option<Self::Item>> {
        let mut module_cache = self.module_cache.borrow_mut();
        if let Some(module) = module_cache.get(module_id) {
            return Ok(Some(module.clone()));
        }

        let state_key = StateKey::module_id(module_id);
        Ok(
            match self
                .state_view
                .get_state_value_bytes(&state_key)
                .map_err(|e| anyhow!("Error retrieving module {:?}: {:?}", module_id, e))?
            {
                Some(bytes) => {
                    let compiled_module =
                        CompiledModule::deserialize_with_config(&bytes, &self.deserializer_config)
                            .map_err(|status| {
                                anyhow!(
                                    "Module {:?} deserialize with error code {:?}",
                                    module_id,
                                    status
                                )
                            })?;

                    let compiled_module = Arc::new(compiled_module);
                    module_cache.insert(module_id.clone(), compiled_module.clone());
                    Some(compiled_module)
                },
                None => None,
            },
        )
    }
}

/// Represents the state used for validation. Stores raw data, module cache and the execution
/// runtime environment. Note that the state can get out-of-date, and it is the responsibility of
/// the owner of the struct to ensure it is up-to-date.
pub struct CachedModuleView<S> {
    /// The raw snapshot of the state used for validation.
    pub state_view: S,
    /// Stores configs needed for execution.
    pub environment: AptosEnvironment,
    /// Versioned cache for deserialized and verified Move modules. The versioning allows to detect
    /// when the version of the code is no longer up-to-date (a newer version has been committed to
    /// the state view) and update the cache accordingly.
    pub module_cache:
        UnsyncModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension, usize>,
}

impl<S: StateView> CachedModuleView<S> {
    /// Creates a new state based on the state view snapshot, with empty module cache and VM
    /// initialized based on configs from the state.
    pub fn new(state_view: S) -> Self {
        info!(
            AdapterLogSchema::new(state_view.id(), 0),
            "Validation environment and module cache created"
        );
        let environment = AptosEnvironment::new(&state_view);
        Self {
            state_view,
            environment,
            module_cache: UnsyncModuleCache::empty(),
        }
    }

    /// Resets the state view snapshot to the new one. Does not invalidate the module cache, nor
    /// the VM.
    pub fn reset_state_view(&mut self, state_view: S) {
        self.state_view = state_view;
    }

    ///  Returns the current state view ID for the caller to decide whether it's compatible with other state views.
    pub fn state_view_id(&self) -> StateViewId {
        self.state_view.id()
    }

    /// Resets the state to the new one, empties module cache, and resets the VM based on the new
    /// state view snapshot.
    pub fn reset_all(&mut self, state_view: S) {
        self.state_view = state_view;
        self.environment = AptosEnvironment::new(&self.state_view);
        self.module_cache = UnsyncModuleCache::empty();
    }

    fn try_override_bytes_and_deserialized_into_compiled_module_with_ext(
        &self,
        mut state_value: StateValue,
        address: &AccountAddress,
        name: &IdentStr,
    ) -> VMResult<(CompiledModule, Arc<AptosModuleExtension>)> {
        // TODO: remove this once framework on mainnet is using the new option module
        if let Some(bytes) = self
            .runtime_environment()
            .get_module_bytes_override(address, name)
        {
            state_value.set_bytes(bytes);
        }
        let compiled_module = self
            .environment
            .runtime_environment()
            .deserialize_into_compiled_module(state_value.bytes())?;
        let extension = Arc::new(AptosModuleExtension::new(state_value));
        Ok((compiled_module, extension))
    }
}

impl<S: StateView> NoOpLayoutCache for CachedModuleView<S> {}

impl<S> WithRuntimeEnvironment for CachedModuleView<S> {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.environment.runtime_environment()
    }
}

impl<S: StateView> ModuleCache for CachedModuleView<S> {
    type Deserialized = CompiledModule;
    type Extension = AptosModuleExtension;
    type Key = ModuleId;
    type Verified = Module;
    type Version = usize;

    fn insert_deserialized_module(
        &self,
        key: Self::Key,
        deserialized_code: Self::Deserialized,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        self.module_cache
            .insert_deserialized_module(key, deserialized_code, extension, version)
    }

    fn insert_verified_module(
        &self,
        key: Self::Key,
        verified_code: Self::Verified,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        self.module_cache
            .insert_verified_module(key, verified_code, extension, version)
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
        // Get the module that exists in cache.
        let (module, version) = match self.module_cache.get_module_or_build_with(key, builder)? {
            None => {
                return Ok(None);
            },
            Some(module_and_version) => module_and_version,
        };

        // Get the state value that exists in the actual state and compute the hash.
        let state_slot = self
            .state_view
            .get_state_slot(&StateKey::module_id(key))
            .map_err(|err| module_storage_error!(key.address(), key.name(), err))?;
        let (value_version, state_value) = match state_slot.into_state_value_and_version_opt() {
            Some((value_version, state_value)) => (value_version as usize, state_value),
            None => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "Module {}::{} cannot be found in storage, but exists in cache",
                            key.address(),
                            key.name()
                        ))
                        .finish(Location::Undefined),
                )
            },
        };
        // deserialize only relies on local config, so only need to detect changes on module bytes
        // if we want to support verified modules, we need
        // to detect changes on aptos environment too.
        Ok(if version == value_version {
            Some((module, version))
        } else {
            let (compiled_module, extension) = self
                .try_override_bytes_and_deserialized_into_compiled_module_with_ext(
                    state_value,
                    key.address(),
                    key.name(),
                )?;

            let new_version = value_version;
            let new_module_code = self.module_cache.insert_deserialized_module(
                key.clone(),
                compiled_module,
                extension,
                new_version,
            )?;
            Some((new_module_code, new_version))
        })
    }

    fn num_modules(&self) -> usize {
        self.module_cache.num_modules()
    }
}
impl<S: StateView> CompiledModuleView for CachedModuleView<S> {
    type Item = Arc<CompiledModule>;

    fn view_compiled_module(&self, module_id: &ModuleId) -> anyhow::Result<Option<Self::Item>> {
        Ok(self.unmetered_get_deserialized_module(module_id.address(), module_id.name())?)
    }
}

impl<S: StateView> ModuleCodeBuilder for CachedModuleView<S> {
    type Deserialized = CompiledModule;
    type Extension = AptosModuleExtension;
    type Key = ModuleId;
    type Verified = Module;

    fn build(
        &self,
        key: &Self::Key,
    ) -> VMResult<Option<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        let state_value = match self
            .state_view
            .get_state_value(&StateKey::module_id(key))
            .map_err(|err| module_storage_error!(key.address(), key.name(), err))?
        {
            Some(state_value) => state_value,
            None => return Ok(None),
        };
        let (compiled_module, extension) = self
            .try_override_bytes_and_deserialized_into_compiled_module_with_ext(
                state_value,
                key.address(),
                key.name(),
            )?;
        let module = ModuleCode::from_deserialized(compiled_module, extension);
        Ok(Some(module))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::state_store::{state_slot::StateSlot, state_value::StateValue, MockStateView};
    use move_binary_format::file_format::empty_module_with_dependencies_and_friends;
    use move_core_types::ident_str;
    use move_vm_runtime::ModuleStorage;
    use std::collections::HashMap;

    fn module_state_value(module: CompiledModule) -> StateValue {
        let mut bytes = vec![];
        module.serialize(&mut bytes).unwrap();
        StateValue::new_legacy(bytes.into())
    }

    #[test]
    fn test_module_cache_consistency() {
        // Have 3 modules in the state.
        let a =
            empty_module_with_dependencies_and_friends("a", vec![], vec![]).set_default_version();
        let b =
            empty_module_with_dependencies_and_friends("b", vec![], vec![]).set_default_version();
        let c =
            empty_module_with_dependencies_and_friends("c", vec![], vec![]).set_default_version();

        let state_view = MockStateView::new(HashMap::from([
            (
                StateKey::module_id(&a.self_id()),
                module_state_value(a.clone()),
            ),
            (
                StateKey::module_id(&b.self_id()),
                module_state_value(b.clone()),
            ),
            (
                StateKey::module_id(&c.self_id()),
                module_state_value(c.clone()),
            ),
        ]));
        let mut state = CachedModuleView::new(state_view);
        assert_eq!(state.module_cache.num_modules(), 0);

        assert!(state
            .unmetered_get_deserialized_module(&AccountAddress::ZERO, ident_str!("d"))
            .unwrap()
            .is_none());
        assert_eq!(
            &a,
            state
                .unmetered_get_deserialized_module(a.self_addr(), a.self_name())
                .unwrap()
                .unwrap()
                .as_ref()
        );
        assert_eq!(
            &c,
            state
                .unmetered_get_deserialized_module(c.self_addr(), c.self_name())
                .unwrap()
                .unwrap()
                .as_ref()
        );

        assert_eq!(state.module_cache.num_modules(), 2);
        assert_eq!(state.module_cache.get_module_version(&a.self_id()), Some(0));
        assert_eq!(state.module_cache.get_module_version(&b.self_id()), None);
        assert_eq!(state.module_cache.get_module_version(&c.self_id()), Some(0));

        // Change module "a" by adding dependencies and also add a new module "d".
        let d =
            empty_module_with_dependencies_and_friends("d", vec![], vec![]).set_default_version();
        let a_new = empty_module_with_dependencies_and_friends("a", vec!["b", "c"], vec![])
            .set_default_version();
        assert_ne!(&a, &a_new);

        let new_state_view = MockStateView::new_with_state_slot(HashMap::from([
            // New code:
            (
                StateKey::module_id(&a_new.self_id()),
                StateSlot::from_db_get(Some((1, module_state_value(a_new.clone())))),
            ),
            (
                StateKey::module_id(&d.self_id()),
                StateSlot::from_db_get(Some((0, module_state_value(d.clone())))),
            ),
            // Old code:
            (
                StateKey::module_id(&b.self_id()),
                StateSlot::from_db_get(Some((0, module_state_value(b.clone())))),
            ),
            (
                StateKey::module_id(&c.self_id()),
                StateSlot::from_db_get(Some((0, module_state_value(c.clone())))),
            ),
        ]));
        state.reset_state_view(new_state_view);

        // New code version should be returned no
        assert_eq!(
            &a_new,
            state
                .unmetered_get_deserialized_module(a_new.self_addr(), a_new.self_name())
                .unwrap()
                .unwrap()
                .as_ref()
        );
        assert_eq!(
            &d,
            state
                .unmetered_get_deserialized_module(d.self_addr(), d.self_name())
                .unwrap()
                .unwrap()
                .as_ref()
        );

        assert_eq!(state.module_cache.num_modules(), 3);
        assert_eq!(state.module_cache.get_module_version(&a.self_id()), Some(1));
        assert_eq!(state.module_cache.get_module_version(&c.self_id()), Some(0));
        assert_eq!(state.module_cache.get_module_version(&d.self_id()), Some(0));

        // Get verified module, to load the transitive closure (modules "b" and "c") as well.
        assert!(state
            .unmetered_get_eagerly_verified_module(a_new.self_addr(), a_new.self_name())
            .unwrap()
            .is_some());
        assert_eq!(state.module_cache.num_modules(), 4);
        assert_eq!(state.module_cache.get_module_version(&a.self_id()), Some(1));
        assert_eq!(state.module_cache.get_module_version(&b.self_id()), Some(0));
        assert_eq!(state.module_cache.get_module_version(&c.self_id()), Some(0));
        assert_eq!(state.module_cache.get_module_version(&d.self_id()), Some(0));
    }
}

// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_logger::info;
use aptos_storage_interface::{
    state_store::state_view::{
        cached_state_view::CachedDbStateView,
        db_state_view::{DbStateView, LatestDbStateCheckpointView},
    },
    DbReader,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountResource,
    state_store::{state_key::StateKey, MoveResourceExt, StateView},
    transaction::{SignedTransaction, VMValidatorResult},
    vm::modules::AptosModuleExtension,
};
use aptos_vm::AptosVM;
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use fail::fail_point;
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    CompiledModule,
};
use move_core_types::{language_storage::ModuleId, vm_status::StatusCode};
use move_vm_runtime::{Module, RuntimeEnvironment, WithRuntimeEnvironment};
use move_vm_types::{
    code::{ModuleCache, ModuleCode, ModuleCodeBuilder, UnsyncModuleCache, WithHash},
    module_storage_error, sha3_256,
};
use rand::{thread_rng, Rng};
use std::sync::{Arc, Mutex};

#[cfg(test)]
#[path = "unit_tests/vm_validator_test.rs"]
mod vm_validator_test;

pub trait TransactionValidation: Send + Sync + Clone {
    type ValidationInstance: aptos_vm::VMValidator;

    /// Validate a txn from client
    fn validate_transaction(&self, _txn: SignedTransaction) -> Result<VMValidatorResult>;

    /// Restart the transaction validation instance
    fn restart(&mut self) -> Result<()>;

    /// Notify about new commit
    fn notify_commit(&mut self);
}

/// Represents the state used for validation. Stores raw data, module cache and the execution
/// runtime environment. Note that the state can get out-of-date, and it is the responsibility of
/// the owner of the struct to ensure it is up-to-date.
struct ValidationState<S> {
    /// The raw snapshot of the state used for validation.
    state_view: S,
    /// Stores configs needed for execution.
    environment: AptosEnvironment,
    /// Versioned cache for deserialized and verified Move modules. The versioning allows to detect
    /// when the version of the code is no longer up-to-date (a newer version has been committed to
    /// the state view) and update the cache accordingly.
    module_cache: UnsyncModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension, usize>,
}

impl<S: StateView> ValidationState<S> {
    /// Creates a new state based on the state view snapshot, with empty module cache and VM
    /// initialized based on configs from the state.
    fn new(state_view: S) -> Self {
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
    fn reset_state_view(&mut self, state_view: S) {
        self.state_view = state_view;
    }

    /// Resets the state to the new one, empties module cache, and resets the VM based on the new
    /// state view snapshot.
    fn reset_all(&mut self, state_view: S) {
        self.state_view = state_view;
        self.environment = AptosEnvironment::new(&self.state_view);
        self.module_cache = UnsyncModuleCache::empty();
    }
}

impl<S> WithRuntimeEnvironment for ValidationState<S> {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.environment.runtime_environment()
    }
}

impl<S: StateView> ModuleCache for ValidationState<S> {
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
        let state_value = self
            .state_view
            .get_state_value(&StateKey::module_id(key))
            .map_err(|err| module_storage_error!(key.address(), key.name(), err))?
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!(
                        "Module {}::{} cannot be found in storage, but exists in cache",
                        key.address(),
                        key.name()
                    ))
                    .finish(Location::Undefined)
            })?;
        let hash = sha3_256(state_value.bytes());

        // If hash is the same - we can use the same module from cache. If the hash is different,
        // then the state contains a newer version of the code. We deserialize the state value and
        // replace the old cache entry with the new code.
        // TODO(loader_v2):
        //   Ideally, commit notification should specify if new modules should be added to the
        //   cache instead of checking the state view bytes. Revisit.
        Ok(if module.extension().hash() == &hash {
            Some((module, version))
        } else {
            let compiled_module = self
                .environment
                .runtime_environment()
                .deserialize_into_compiled_module(state_value.bytes())?;
            let extension = Arc::new(AptosModuleExtension::new(state_value));

            let new_version = version + 1;
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

impl<S: StateView> ModuleCodeBuilder for ValidationState<S> {
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
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        let compiled_module = self
            .runtime_environment()
            .deserialize_into_compiled_module(state_value.bytes())?;
        let extension = Arc::new(AptosModuleExtension::new(state_value));
        let module = ModuleCode::from_deserialized(compiled_module, extension);
        Ok(Some(module))
    }
}

struct VMValidator {
    db_reader: Arc<dyn DbReader>,
    state: ValidationState<CachedDbStateView>,
}

impl Clone for VMValidator {
    fn clone(&self) -> Self {
        Self::new(self.db_reader.clone())
    }
}

impl VMValidator {
    fn new(db_reader: Arc<dyn DbReader>) -> Self {
        let db_state_view = db_reader
            .latest_state_checkpoint_view()
            .expect("Get db view cannot fail");
        VMValidator {
            db_reader,
            state: ValidationState::new(db_state_view.into()),
        }
    }

    fn db_state_view(&self) -> DbStateView {
        self.db_reader
            .latest_state_checkpoint_view()
            .expect("Get db view cannot fail")
    }

    fn restart(&mut self) -> Result<()> {
        let db_state_view = self.db_state_view();
        self.state.reset_all(db_state_view.into());
        Ok(())
    }

    fn notify_commit(&mut self) {
        let db_state_view = self.db_state_view();

        // On commit, we need to update the state view so that we can see the latest resources.
        self.state.reset_state_view(db_state_view.into());
    }
}

/// returns account's sequence number from storage
pub fn get_account_sequence_number(
    state_view: &DbStateView,
    address: AccountAddress,
) -> Result<u64> {
    fail_point!("vm_validator::get_account_sequence_number", |_| {
        Err(anyhow::anyhow!(
            "Injected error in get_account_sequence_number"
        ))
    });

    match AccountResource::fetch_move_resource(state_view, &address)? {
        Some(account_resource) => Ok(account_resource.sequence_number()),
        None => Ok(0),
    }
}

// A pool of VMValidators that can be used to validate transactions concurrently. This is done because
// the VM is not thread safe today. This is a temporary solution until the VM is made thread safe.
// TODO(loader_v2): Re-implement because VM is thread-safe now.
#[derive(Clone)]
pub struct PooledVMValidator {
    vm_validators: Vec<Arc<Mutex<VMValidator>>>,
}

impl PooledVMValidator {
    pub fn new(db_reader: Arc<dyn DbReader>, pool_size: usize) -> Self {
        let mut vm_validators = Vec::new();
        for _ in 0..pool_size {
            vm_validators.push(Arc::new(Mutex::new(VMValidator::new(db_reader.clone()))));
        }
        PooledVMValidator { vm_validators }
    }

    fn get_next_vm(&self) -> Arc<Mutex<VMValidator>> {
        let mut rng = thread_rng(); // Create a thread-local random number generator
        let random_index = rng.gen_range(0, self.vm_validators.len()); // Generate random index
        self.vm_validators[random_index].clone() // Return the VM at the random index
    }
}

impl TransactionValidation for PooledVMValidator {
    type ValidationInstance = AptosVM;

    fn validate_transaction(&self, txn: SignedTransaction) -> Result<VMValidatorResult> {
        let vm_validator = self.get_next_vm();

        fail_point!("vm_validator::validate_transaction", |_| {
            Err(anyhow::anyhow!(
                "Injected error in vm_validator::validate_transaction"
            ))
        });

        let vm_validator_locked = vm_validator.lock().unwrap();

        use aptos_vm::VMValidator;
        let vm = AptosVM::new(
            &vm_validator_locked.state.environment,
            &vm_validator_locked.state.state_view,
        );
        Ok(vm.validate_transaction(
            txn,
            &vm_validator_locked.state.state_view,
            &vm_validator_locked.state,
        ))
    }

    fn restart(&mut self) -> Result<()> {
        for vm_validator in &self.vm_validators {
            vm_validator.lock().unwrap().restart()?;
        }
        Ok(())
    }

    fn notify_commit(&mut self) {
        for vm_validator in &self.vm_validators {
            vm_validator.lock().unwrap().notify_commit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::state_store::{state_value::StateValue, MockStateView};
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
        let mut state = ValidationState::new(state_view);
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

        let new_state_view = MockStateView::new(HashMap::from([
            // New code:
            (
                StateKey::module_id(&a_new.self_id()),
                module_state_value(a_new.clone()),
            ),
            (
                StateKey::module_id(&d.self_id()),
                module_state_value(d.clone()),
            ),
            // Old code:
            (
                StateKey::module_id(&b.self_id()),
                module_state_value(b.clone()),
            ),
            (
                StateKey::module_id(&c.self_id()),
                module_state_value(c.clone()),
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

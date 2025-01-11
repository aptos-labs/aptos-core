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
    state_store::{state_key::StateKey, MoveResourceExt, StateView, TStateView},
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

struct VMValidator {
    db_reader: Arc<dyn DbReader>,
    state_view: CachedDbStateView,
    /// Versioned cache for deserialized and verified Move modules. The versioning allows to detect
    /// when the version of the code is no longer up-to-date (a newer version has been committed to
    /// the state view) and update the cache accordingly.
    module_cache: UnsyncModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension, usize>,
    vm: AptosVM,
}

impl WithRuntimeEnvironment for VMValidator {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.vm.runtime_environment()
    }
}

impl ModuleCache for VMValidator {
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
        let (module, version) = match self.module_cache.get_module_or_build_with(key, builder)? {
            None => {
                return Ok(None);
            },
            Some(module_and_version) => module_and_version,
        };

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
        Ok(if module.extension().hash() == &hash {
            Some((module, version))
        } else {
            let compiled_module = self
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

impl ModuleCodeBuilder for VMValidator {
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

impl Clone for VMValidator {
    fn clone(&self) -> Self {
        Self::new(self.db_reader.clone())
    }
}

impl VMValidator {
    fn new_vm_for_validation(state_view: &impl StateView) -> AptosVM {
        info!(
            AdapterLogSchema::new(state_view.id(), 0),
            "AptosVM created for Validation"
        );
        let env = AptosEnvironment::new(state_view);
        AptosVM::new(env, state_view)
    }

    fn new(db_reader: Arc<dyn DbReader>) -> Self {
        let db_state_view = db_reader
            .latest_state_checkpoint_view()
            .expect("Get db view cannot fail");
        let vm = Self::new_vm_for_validation(&db_state_view);

        VMValidator {
            db_reader,
            state_view: db_state_view.into(),
            module_cache: UnsyncModuleCache::empty(),
            vm,
        }
    }

    fn db_state_view(&self) -> DbStateView {
        self.db_reader
            .latest_state_checkpoint_view()
            .expect("Get db view cannot fail")
    }

    fn restart(&mut self) -> Result<()> {
        let db_state_view = self.db_state_view();
        self.state_view = db_state_view.into();

        // If restarting, configs must have changed, so we need to empty the cache and re-create
        // the VM.
        self.module_cache = UnsyncModuleCache::empty();
        self.vm = Self::new_vm_for_validation(&self.state_view);

        Ok(())
    }

    fn notify_commit(&mut self) {
        let db_state_view = self.db_state_view();
        self.state_view = db_state_view.into();
        // We do not update module cache here - it will update itself if needed when reading the
        // module.
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
        Ok(vm_validator_locked.vm.validate_transaction(
            txn,
            &vm_validator_locked.state_view,
            &*vm_validator_locked,
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

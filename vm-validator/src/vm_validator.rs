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
    state_store::{MoveResourceExt, StateView},
    transaction::{SignedTransaction, VMValidatorResult},
};
use aptos_vm::AptosVM;
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::module_and_script_storage::{AptosCodeStorageAdapter, AsAptosCodeStorage};
use fail::fail_point;
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
    module_storage: AptosCodeStorageAdapter<'static, CachedDbStateView, AptosEnvironment>,
    vm: AptosVM,
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
        let state_view = CachedDbStateView::from(db_state_view.clone());
        let module_storage =
            CachedDbStateView::from(db_state_view).into_aptos_code_storage(vm.environment());

        VMValidator {
            db_reader,
            state_view,
            module_storage,
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

        self.state_view = db_state_view.clone().into();
        self.vm = Self::new_vm_for_validation(&self.state_view);
        self.module_storage =
            CachedDbStateView::from(db_state_view).into_aptos_code_storage(self.vm.environment());

        Ok(())
    }

    fn notify_commit(&mut self) {
        let db_state_view = self.db_state_view();
        self.state_view = db_state_view.into();
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
        info!("pooled_validate_transaction (address: {:?}, replay_protector: {:?}, expiration_timestamp_secs: {:?})", txn.sender(), txn.replay_protector(), txn.expiration_timestamp_secs());
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
            &vm_validator_locked.module_storage,
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

// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use anyhow::Result;
use aptos_logger::info;
use aptos_storage_interface::{
    cached_state_view::CachedDbStateView,
    state_view::{DbStateView, LatestDbStateCheckpointView},
    DbReader,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountResource,
    state_store::{MoveResourceExt, StateView},
    transaction::{SignedTransaction, VMValidatorResult},
};
use aptos_vm::AptosVM;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use fail::fail_point;
use rand::{thread_rng, Rng};
use std::sync::{Arc, Mutex};
use aptos_types::transaction::EntryFunction;
use aptos_vm::move_vm_ext::SessionId;

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

pub struct VMValidator {
    db_reader: Arc<dyn DbReader>,
    state_view: CachedDbStateView,
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
        AptosVM::new(state_view)
    }

    pub fn new(db_reader: Arc<dyn DbReader>) -> Self {
        let db_state_view = db_reader
            .latest_state_checkpoint_view()
            .expect("Get db view cannot fail");

        let vm = Self::new_vm_for_validation(&db_state_view);
        VMValidator {
            db_reader,
            state_view: db_state_view.into(),
            vm,
        }
    }

    pub fn check_randomness(&self, txn: &SignedTransaction) -> bool {
        let resolver = self.vm.as_move_resolver(&self.state_view);
        let mut session = self.vm.new_session(&resolver, SessionId::Void, None);
        self.vm.check_randomness(txn, &resolver, &mut session, &HashSet::new()).0
    }

    pub fn check_randomness_in_batch(&self, txn: &Vec<SignedTransaction>, entry_sets: &HashSet<EntryFunction>) -> (bool, Option<EntryFunction>) {
        let resolver = self.vm.as_move_resolver(&self.state_view);
        self.vm.check_randomness_in_batch(txn, &resolver, entry_sets)
    }

}

impl TransactionValidation for VMValidator {
    type ValidationInstance = AptosVM;

    fn validate_transaction(&self, txn: SignedTransaction) -> Result<VMValidatorResult> {
        fail_point!("vm_validator::validate_transaction", |_| {
            Err(anyhow::anyhow!(
                "Injected error in vm_validator::validate_transaction"
            ))
        });
        use aptos_vm::VMValidator;

        Ok(self.vm.validate_transaction(txn, &self.state_view))
    }

    fn restart(&mut self) -> Result<()> {
        self.notify_commit();

        self.vm = Self::new_vm_for_validation(&self.state_view);
        Ok(())
    }

    fn notify_commit(&mut self) {
        self.state_view = self
            .db_reader
            .latest_state_checkpoint_view()
            .expect("Get db view cannot fail")
            .into();
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

    pub fn get_next_vm(&self) -> Arc<Mutex<VMValidator>> {
        let mut rng = thread_rng(); // Create a thread-local random number generator
        let random_index = rng.gen_range(0, self.vm_validators.len()); // Generate random index
        self.vm_validators[random_index].clone() // Return the VM at the random index
    }

    pub fn check_randomness(&self, txn: &SignedTransaction) -> bool {
        self.get_next_vm().lock().unwrap().check_randomness(txn)
    }

    pub fn check_randomness_for_batch_txns(&self, txn: &Vec<SignedTransaction>, entry_sets: &HashSet<EntryFunction>) -> (bool, Option<EntryFunction>) {
        self.get_next_vm().lock().unwrap().check_randomness_in_batch(txn, entry_sets)
    }

    pub fn check_randomness_in_batch(&self, txns: &Option<Vec<SignedTransaction>>, entry_sets: &HashSet<EntryFunction>) -> (bool, HashSet<EntryFunction>) {
        txns.as_ref()
            .map_or((false, HashSet::new()), |txns| {
                let (res, entry_opt) = self.check_randomness_for_batch_txns(txns, entry_sets);
                let mut entry_set = HashSet::new();
                //for entry_opt in entries.iter() {
                    if let Some(entry) = entry_opt {
                        if !entry_set.contains(&entry) {
                            entry_set.insert(entry.clone());
                        }
                    }
                //}
                (res, entry_set)
            })
    }

    // pub fn check_randomness_in_batch(&self, txns: &Option<Vec<SignedTransaction>>) -> (bool, HashSet<EntryFunction>) {
    //     txns.as_ref()
    //         .map_or((false, HashSet::new()), |txns| {
    //             let (res, entry_opt) = self.check_randomness_for_batch_txns(txns);
    //             let mut entry_set = HashSet::new();
    //             //for entry_opt in entries.iter() {
    //             if let Some(entry) = entry_opt {
    //                 if !entry_set.contains(&entry) {
    //                     entry_set.insert(entry.clone());
    //                 }
    //             }
    //             //}
    //             (res, entry_set)
    //         })
    // }

}

impl TransactionValidation for PooledVMValidator {
    type ValidationInstance = AptosVM;

    fn validate_transaction(&self, txn: SignedTransaction) -> Result<VMValidatorResult> {
        self.get_next_vm().lock().unwrap().validate_transaction(txn)
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

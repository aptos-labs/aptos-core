// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_state_view::{account_with_state_view::AsAccountWithStateView, StateViewId};
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountSequenceInfo,
    account_view::AccountView,
    on_chain_config::OnChainConfigPayload,
    transaction::{SignedTransaction, VMValidatorResult},
};
use aptos_vm::AptosVM;
use executor::components::in_memory_state_calculator::IntoLedgerView;
use fail::fail_point;
use std::sync::Arc;
use storage_interface::{
    state_view::LatestDbStateView, verified_state_view::VerifiedStateView, DbReader,
};

#[cfg(test)]
#[path = "unit_tests/vm_validator_test.rs"]
mod vm_validator_test;

pub trait TransactionValidation: Send + Sync + Clone {
    type ValidationInstance: aptos_vm::VMValidator;

    /// Validate a txn from client
    fn validate_transaction(&self, _txn: SignedTransaction) -> Result<VMValidatorResult>;

    /// Restart the transaction validation instance
    fn restart(&mut self, config: OnChainConfigPayload) -> Result<()>;

    /// Notify about new commit
    fn notify_commit(&mut self);
}

fn latest_state_view(db_reader: &Arc<dyn DbReader>) -> VerifiedStateView {
    let ledger_view = db_reader
        .get_latest_tree_state()
        .expect("Should not fail.")
        .into_ledger_view(db_reader)
        .expect("Should not fail.");

    ledger_view.state_view(
        &ledger_view,
        StateViewId::TransactionValidation {
            base_version: ledger_view.version().expect("Must be bootstrapped."),
        },
        db_reader.clone(),
    )
}

pub struct VMValidator {
    db_reader: Arc<dyn DbReader>,
    cached_state_view: VerifiedStateView,
    vm: AptosVM,
}

impl Clone for VMValidator {
    fn clone(&self) -> Self {
        Self::new(self.db_reader.clone())
    }
}

impl VMValidator {
    pub fn new(db_reader: Arc<dyn DbReader>) -> Self {
        let cached_state_view = latest_state_view(&db_reader);

        let vm = AptosVM::new_for_validation(&cached_state_view);
        VMValidator {
            db_reader,
            cached_state_view,
            vm,
        }
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

        Ok(self.vm.validate_transaction(txn, &self.cached_state_view))
    }

    fn restart(&mut self, _config: OnChainConfigPayload) -> Result<()> {
        self.notify_commit();

        self.vm = AptosVM::new_for_validation(&self.cached_state_view);
        Ok(())
    }

    fn notify_commit(&mut self) {
        self.cached_state_view = latest_state_view(&self.db_reader);
    }
}

/// returns account's sequence number from storage
pub fn get_account_sequence_number(
    storage: Arc<dyn DbReader>,
    address: AccountAddress,
) -> Result<AccountSequenceInfo> {
    fail_point!("vm_validator::get_account_sequence_number", |_| {
        Err(anyhow::anyhow!(
            "Injected error in get_account_sequence_number"
        ))
    });
    let db_state_view = storage.latest_state_view()?;

    let account_state_view = db_state_view.as_account_with_state_view(&address);

    if let Ok(Some(crsn)) = account_state_view.get_crsn_resource() {
        return Ok(AccountSequenceInfo::CRSN {
            min_nonce: crsn.min_nonce(),
            size: crsn.size(),
        });
    }

    match account_state_view.get_account_resource()? {
        Some(account_resource) => Ok(AccountSequenceInfo::Sequential(
            account_resource.sequence_number(),
        )),
        None => Ok(AccountSequenceInfo::Sequential(0)),
    }
}

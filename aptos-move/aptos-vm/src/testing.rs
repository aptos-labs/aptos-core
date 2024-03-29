// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::AptosVM;
#[cfg(any(test, feature = "testing"))]
use crate::{
    aptos_vm::get_or_vm_startup_failure, data_cache::AsMoveResolver,
    transaction_metadata::TransactionMetadata,
};
#[cfg(any(test, feature = "testing"))]
use aptos_types::{state_store::StateView, transaction::SignedTransaction};
#[cfg(any(test, feature = "testing"))]
use aptos_vm_logging::log_schema::AdapterLogSchema;
#[cfg(any(test, feature = "testing"))]
use aptos_vm_types::{change_set::VMChangeSet, output::VMOutput};
use move_binary_format::errors::VMResult;
#[cfg(any(test, feature = "testing"))]
use move_core_types::vm_status::VMStatus;

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum InjectedError {
    EndOfRunEpilogue,
}

pub(crate) fn maybe_raise_injected_error(_error_type: InjectedError) -> VMResult<()> {
    #[cfg(feature = "testing")]
    {
        testing_only::maybe_raise_injected_error(_error_type)
    }

    #[cfg(not(feature = "testing"))]
    Ok(())
}

#[cfg(feature = "testing")]
pub mod testing_only {
    use super::InjectedError;
    use move_binary_format::errors::{Location, PartialVMError, VMResult};
    use move_core_types::vm_status::StatusCode;
    use std::{cell::RefCell, collections::HashSet};

    thread_local! {
        static INJECTED_ERRORS: RefCell<HashSet<InjectedError >> = RefCell::new(HashSet::new());
    }

    pub(crate) fn maybe_raise_injected_error(error_type: InjectedError) -> VMResult<()> {
        match INJECTED_ERRORS.with(|injected_errors| injected_errors.borrow_mut().take(&error_type))
        {
            None => Ok(()),
            Some(_) => Err(PartialVMError::new(
                StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
            )
            .with_message(format!("injected error: {:?}", error_type))
            .finish(Location::Undefined)),
        }
    }

    pub fn inject_error_once(error_type: InjectedError) {
        INJECTED_ERRORS.with(|injected_errors| {
            injected_errors.borrow_mut().insert(error_type);
        })
    }
}

impl AptosVM {
    #[cfg(any(test, feature = "testing"))]
    pub fn test_failed_transaction_cleanup(
        &self,
        error_vm_status: VMStatus,
        txn: &SignedTransaction,
        state_view: &impl StateView,
        gas_meter_balance: u64,
    ) -> (VMStatus, VMOutput) {
        let txn_data = TransactionMetadata::new(txn);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let mut gas_meter = self
            .make_standard_gas_meter(gas_meter_balance.into(), &log_context)
            .expect("Should be able to create a gas meter for tests");
        let change_set_configs = &get_or_vm_startup_failure(&self.storage_gas_params, &log_context)
            .expect("Storage gas parameters should exist for tests")
            .change_set_configs;

        let resolver = state_view.as_move_resolver();
        self.failed_transaction_cleanup(
            VMChangeSet::empty(),
            error_vm_status,
            &mut gas_meter,
            &txn_data,
            &resolver,
            &log_context,
            change_set_configs,
        )
    }
}

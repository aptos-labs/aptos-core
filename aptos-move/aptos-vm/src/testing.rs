// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::AptosVM;
#[cfg(any(test, feature = "testing"))]
use crate::{
    aptos_vm::get_or_vm_startup_failure, data_cache::AsMoveResolver,
    move_vm_ext::session::user_transaction_sessions::session_change_sets::SystemSessionChangeSet,
    transaction_metadata::TransactionMetadata,
};
#[cfg(any(test, feature = "testing"))]
use aptos_types::{state_store::StateView, transaction::SignedTransaction};
#[cfg(any(test, feature = "testing"))]
use aptos_vm_logging::log_schema::AdapterLogSchema;
#[cfg(any(test, feature = "testing"))]
use aptos_vm_types::output::VMOutput;
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
        use crate::gas::make_prod_gas_meter;
        use move_vm_runtime::module_traversal::{TraversalContext, TraversalStorage};

        let txn_data = TransactionMetadata::new(txn);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let vm_gas_params = self
            .gas_params()
            .expect("should be able to get gas params")
            .vm
            .clone();
        let storage_gas_params = self
            .storage_gas_params
            .as_ref()
            .expect("should be able to get storage gas params")
            .clone();

        let mut gas_meter = make_prod_gas_meter(
            self.gas_feature_version,
            vm_gas_params,
            storage_gas_params,
            false,
            gas_meter_balance.into(),
        );

        let change_set_configs = &get_or_vm_startup_failure(&self.storage_gas_params, &log_context)
            .expect("Storage gas parameters should exist for tests")
            .change_set_configs;

        let resolver = state_view.as_move_resolver();
        let storage = TraversalStorage::new();
        self.failed_transaction_cleanup(
            SystemSessionChangeSet::empty(),
            error_vm_status,
            &mut gas_meter,
            &txn_data,
            &resolver,
            &log_context,
            change_set_configs,
            &mut TraversalContext::new(&storage),
        )
    }
}

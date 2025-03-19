// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(test, feature = "testing"))]
use crate::{
    aptos_vm::{serialized_signer, SerializedSigners},
    transaction_metadata::TransactionMetadata,
};
use crate::{data_cache_v2::Session, move_vm_ext::SessionId, AptosVM};
#[cfg(any(test, feature = "testing"))]
use aptos_types::{
    state_store::StateView, transaction::SignedTransaction,
    vm::state_view_adapter::ExecutorViewAdapter,
};
#[cfg(any(test, feature = "testing"))]
use aptos_vm_logging::log_schema::AdapterLogSchema;
#[cfg(any(test, feature = "testing"))]
use aptos_vm_types::{
    module_and_script_storage::AsAptosCodeStorage, output::VMOutput,
    resolver::NoopBlockSynchronizationKillSwitch,
};
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
            .gas_params_for_test()
            .expect("should be able to get gas params")
            .vm
            .clone();
        let storage_gas_params = self
            .storage_gas_params(&log_context)
            .expect("should be able to get storage gas params")
            .clone();

        let mut gas_meter = make_prod_gas_meter(
            self.gas_feature_version(),
            vm_gas_params,
            storage_gas_params,
            false,
            gas_meter_balance.into(),
            &NoopBlockSynchronizationKillSwitch {},
        );

        let change_set_configs = self
            .storage_gas_params(&log_context)
            .expect("Storage gas parameters should exist for tests")
            .change_set_configs
            .clone();

        let executor_view = ExecutorViewAdapter::borrowed(state_view);
        let module_storage = state_view.as_aptos_code_storage(self.runtime_environment());
        let session = Session::new(
            &executor_view,
            &module_storage,
            self.environment(),
            SessionId::prologue_meta(&txn_data),
            Some(txn_data.as_user_transaction_context()),
            change_set_configs,
        );

        let traversal_storage = TraversalStorage::new();
        self.failed_transaction_cleanup(
            session,
            error_vm_status,
            &mut gas_meter,
            &txn_data,
            &SerializedSigners::new(
                vec![serialized_signer(&txn_data.sender)],
                txn_data.fee_payer().as_ref().map(serialized_signer),
            ),
            &log_context,
            &mut TraversalContext::new(&traversal_storage),
        )
    }
}

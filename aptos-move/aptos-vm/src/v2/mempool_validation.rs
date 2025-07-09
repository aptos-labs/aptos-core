// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{TRANSACTIONS_VALIDATED, TXN_VALIDATION_SECONDS},
    data_cache::AsMoveResolver,
    gas::make_prod_gas_meter,
    v2::vm::AptosVMv2,
    VMValidator,
};
use aptos_types::{
    state_store::StateView,
    transaction::{SignedTransaction, VMValidatorResult},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage,
    resolver::NoopBlockSynchronizationKillSwitch,
};
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::module_traversal::{TraversalContext, TraversalStorage};

impl VMValidator for AptosVMv2 {
    fn validate_transaction(
        &self,
        txn: SignedTransaction,
        state_view: &impl StateView,
        code_view: &impl AptosModuleStorage,
    ) -> VMValidatorResult {
        let _timer = TXN_VALIDATION_SECONDS.start_timer();
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        // TODO: Some logic for zk / keyless / auth here

        let txn = match txn.check_signature() {
            Ok(txn) => txn,
            Err(_) => {
                return VMValidatorResult::error(StatusCode::INVALID_SIGNATURE);
            },
        };

        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let data_view = state_view.as_move_resolver();
        let mut session = match self.new_user_transaction_session(
            &data_view,
            code_view,
            &log_context,
            &mut traversal_context,
            &txn,
        ) {
            Ok(session) => session,
            Err(status) => {
                return VMValidatorResult::new(Some(status.status_code()), 0);
            },
        };

        let mut gas_meter = session.build_gas_meter(
            |gas_feature_version,
             vm_gas_params,
             storage_gas_params,
             is_approved_gov_script,
             meter_balance,
             _| {
                make_prod_gas_meter(
                    gas_feature_version,
                    vm_gas_params,
                    storage_gas_params,
                    is_approved_gov_script,
                    meter_balance,
                    &NoopBlockSynchronizationKillSwitch {},
                )
            },
        );

        let (counter_label, result) =
            match session.execute_user_transaction_prologue(&mut gas_meter) {
                Err(err) if err.status_code() != StatusCode::SEQUENCE_NUMBER_TOO_NEW => (
                    "failure",
                    VMValidatorResult::new(Some(err.status_code()), 0),
                ),
                _ => (
                    "success",
                    VMValidatorResult::new(None, txn.gas_unit_price()),
                ),
            };
        TRANSACTIONS_VALIDATED
            .with_label_values(&[counter_label])
            .inc();

        result
    }
}

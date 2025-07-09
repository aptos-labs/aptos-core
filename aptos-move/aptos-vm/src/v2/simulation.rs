// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::AsMoveResolver,
    gas::make_prod_gas_meter,
    v2::{loader::AptosLoader, AptosVMv2},
};
use aptos_types::{
    state_store::StateView,
    transaction::{AuxiliaryInfo, AuxiliaryInfoTrait, SignedTransaction, TransactionOutput},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::AsAptosCodeStorage, output::VMOutput,
    resolver::NoopBlockSynchronizationKillSwitch,
};
use claims::assert_err;
use move_core_types::vm_status::VMStatus;
use move_vm_runtime::{
    dispatch_loader,
    module_traversal::{TraversalContext, TraversalStorage},
    ScriptLoader,
};

pub(crate) struct AptosSimulationVMv2 {
    vm: AptosVMv2,
}

impl AptosSimulationVMv2 {
    pub(crate) fn new(environment: &AptosEnvironment) -> Self {
        let mut vm = AptosVMv2::new(environment);
        vm.is_simulation = true;
        Self { vm }
    }

    pub(crate) fn simulate_user_transaction(
        &self,
        txn: &SignedTransaction,
        state_view: &impl StateView,
    ) -> (VMStatus, TransactionOutput) {
        assert_err!(
            txn.verify_signature(),
            "Simulated transaction should not have a valid signature"
        );

        let code_view = state_view.as_aptos_code_storage(&self.vm.environment);
        dispatch_loader!(&code_view, loader, {
            self.simulate_user_transaction_impl(txn, state_view, &loader)
        })
    }

    pub(crate) fn simulate_user_transaction_impl(
        &self,
        txn: &SignedTransaction,
        state_view: &impl StateView,
        loader: &(impl AptosLoader + ScriptLoader),
    ) -> (VMStatus, TransactionOutput) {
        let data_view = state_view.as_move_resolver();
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let simulation_result = self.vm.execute_user_transaction_with_custom_gas_meter(
            &data_view,
            loader,
            &NoopBlockSynchronizationKillSwitch {},
            &log_context,
            &mut traversal_context,
            txn,
            make_prod_gas_meter,
            // For simulation, we use empty info.
            &AuxiliaryInfo::new_empty(),
        );

        let (status, output) = match simulation_result {
            Ok((output, _)) => (VMStatus::Executed, output),
            Err(status) => {
                let output = VMOutput::discarded(status.status_code());
                (status, output)
            },
        };
        let output = output
            .try_materialize_into_transaction_output(&data_view)
            .expect("Materializing aggregator V1 deltas should never fail");

        (status, output)
    }
}

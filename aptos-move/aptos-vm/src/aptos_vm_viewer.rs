// Copyright (c) 2024 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::aptos_vm::get_or_vm_startup_failure;
use crate::gas::{make_prod_gas_meter, ProdGasMeter};
use crate::move_vm_ext::SessionId::Void;
use crate::AptosVM;
use aptos_types::state_store::StateView;
use aptos_types::transaction::{ViewFunction, ViewFunctionOutput};
use aptos_vm_logging::log_schema::AdapterLogSchema;

/// Move VM with only view function API.
/// Convenient to use when more than one view function needs to be executed on the same state-view,
/// as it avoids to set up AptosVM upon each function execution.
pub struct AptosVMViewer<'t, SV: StateView> {
    vm: AptosVM,
    state_view: &'t SV,
    log_context: AdapterLogSchema,
}

impl<'t, SV: StateView> AptosVMViewer<'t, SV> {
    /// Creates a new VM instance, initializing the runtime environment from the state.
    pub fn new(state_view: &'t SV) -> Self {
        let vm = AptosVM::new(state_view);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);
        Self {
            vm,
            state_view,
            log_context,
        }
    }

    fn create_gas_meter(&self, max_gas_amount: u64) -> anyhow::Result<ProdGasMeter> {
        let vm_gas_params =
            match get_or_vm_startup_failure(&self.vm.gas_params_internal(), &self.log_context) {
                Ok(gas_params) => gas_params.vm.clone(),
                Err(err) => return Err(anyhow::Error::msg(format!("{}", err))),
            };
        let storage_gas_params =
            match get_or_vm_startup_failure(&self.vm.storage_gas_params, &self.log_context) {
                Ok(gas_params) => gas_params.clone(),
                Err(err) => return Err(anyhow::Error::msg(format!("{}", err))),
            };

        let gas_meter = make_prod_gas_meter(
            self.vm.gas_feature_version,
            vm_gas_params,
            storage_gas_params,
            /* is_approved_gov_script */ false,
            max_gas_amount.into(),
        );
        Ok(gas_meter)
    }

    pub fn execute_view_function(
        &self,
        function: ViewFunction,
        max_gas_amount: u64,
    ) -> ViewFunctionOutput {
        let resolver = self.vm.as_move_resolver(self.state_view);
        let mut session = self.vm.new_session(&resolver, Void, None);
        let mut gas_meter = match self.create_gas_meter(max_gas_amount) {
            Ok(meter) => meter,
            Err(e) => return ViewFunctionOutput::new(Err(e), 0),
        };
        let (module_id, func_name, type_args, arguments) = function.into_inner();

        let execution_result = AptosVM::execute_view_function_in_vm(
            &mut session,
            &self.vm,
            module_id,
            func_name,
            type_args,
            arguments,
            &mut gas_meter,
        );
        let gas_used = AptosVM::gas_used(max_gas_amount.into(), &gas_meter);
        match execution_result {
            Ok(result) => ViewFunctionOutput::new(Ok(result), gas_used),
            Err(e) => ViewFunctionOutput::new(Err(e), gas_used),
        }
    }
}

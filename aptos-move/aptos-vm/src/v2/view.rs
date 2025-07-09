// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::AsMoveResolver,
    gas::make_prod_gas_meter,
    move_vm_ext::SessionId,
    v2::{AptosSession, AptosVMv2},
};
use aptos_types::{
    state_store::StateView, transaction::ViewFunctionOutput, vm::module_metadata::get_metadata,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::module_and_script_storage::AsAptosCodeStorage;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use move_vm_runtime::{
    module_traversal::{TraversalContext, TraversalStorage},
    ModuleStorage,
};

pub(crate) struct AptosViewVMv2 {
    vm: AptosVMv2,
}

impl AptosViewVMv2 {
    pub(crate) fn new(environment: &AptosEnvironment) -> Self {
        let vm = AptosVMv2::new(environment);
        Self { vm }
    }

    pub(crate) fn execute_view_function(
        &self,
        state_view: &impl StateView,
        module_id: ModuleId,
        func_name: Identifier,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        max_gas_amount: u64,
    ) -> ViewFunctionOutput {
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let data_view = state_view.as_move_resolver();
        let code_view = state_view.as_aptos_code_storage(&self.vm.environment);

        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let mut session = self
            .vm
            .new_system_session(
                &data_view,
                &code_view,
                &log_context,
                &mut traversal_context,
                SessionId::Void,
            )
            .expect("TODO: error handling");
        let mut gas_meter = session.build_gas_meter(make_prod_gas_meter, max_gas_amount.into());

        let func = session
            .code_view
            .load_function(&module_id, &func_name, &ty_args)
            .expect("TODO: error handling");
        let module = func
            .owner_as_module()
            .expect("Function must be owned by a module");

        let _metadata = get_metadata(&module.metadata);
        // TODO: validate view function

        let _result = session
            .execute_loaded_function(func, args, &mut gas_meter)
            .expect("TODO: error handling");

        unimplemented!()
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::AsMoveResolver,
    gas::make_prod_gas_meter,
    move_vm_ext::{AptosMoveResolver, SessionId},
    v2::{
        session::{gas_used, Session},
        AptosSession, AptosVMv2,
    },
};
use aptos_types::{state_store::StateView, transaction::ViewFunctionOutput};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::AsAptosCodeStorage, resolver::NoopBlockSynchronizationKillSwitch,
};
use move_binary_format::errors::VMResult;
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
};
use move_vm_runtime::{
    dispatch_loader,
    module_traversal::{TraversalContext, TraversalStorage},
    LegacyLoaderConfig, Loader,
};
use move_vm_types::gas::GasMeter;

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
        let code_view = state_view.as_aptos_code_storage(&self.vm.environment);
        dispatch_loader!(&code_view, loader, {
            self.execute_view_function_with_loader(
                state_view,
                &loader,
                module_id,
                func_name,
                ty_args,
                args,
                max_gas_amount,
            )
        })
    }

    pub(crate) fn execute_view_function_with_loader(
        &self,
        state_view: &impl StateView,
        loader: &impl Loader,
        module_id: ModuleId,
        func_name: Identifier,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        max_gas_amount: u64,
    ) -> ViewFunctionOutput {
        let data_view = state_view.as_move_resolver();
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let mut session = match self.vm.new_system_session(
            &data_view,
            loader,
            &log_context,
            &mut traversal_context,
            SessionId::Void,
        ) {
            Ok(session) => session,
            Err(status) => {
                return ViewFunctionOutput::new_error_message(
                    format!("Failed to create a view session: {}", status),
                    Some(status.status_code()),
                    0,
                )
            },
        };

        let mut gas_meter = session.build_gas_meter(
            make_prod_gas_meter,
            max_gas_amount.into(),
            &NoopBlockSynchronizationKillSwitch {},
        );
        let result = self.execute_view_function_impl(
            &mut session,
            &mut gas_meter,
            &module_id,
            &func_name,
            &ty_args,
            args,
        );
        let gas_used = gas_used(max_gas_amount.into(), &gas_meter);

        match result {
            Ok(returned_bytes) => ViewFunctionOutput::new_ok(returned_bytes, gas_used),
            Err(_) => {
                // TODO(aptos-vm-v2):
                //   This does not support this functionality end to end. For Move aborts, we need
                //   to inject abort info.
                unimplemented!("View function error path i snot implemented")
            },
        }
    }
}

impl AptosViewVMv2 {
    fn execute_view_function_impl(
        &self,
        session: &mut Session<impl AptosMoveResolver, impl Loader>,
        gas_meter: &mut impl GasMeter,
        module_id: &ModuleId,
        func_name: &IdentStr,
        ty_args: &[TypeTag],
        args: Vec<Vec<u8>>,
    ) -> VMResult<Vec<Vec<u8>>> {
        let func = session.loader.load_instantiated_function(
            &LegacyLoaderConfig::unmetered(),
            gas_meter,
            session.traversal_context,
            module_id,
            func_name,
            ty_args,
        )?;

        // TODO(aptos-vm-v2):
        //   1. Validate transaction arguments for view function.
        //   2. Consider enabling struct constructors at all times, or alternatives like pre-
        //      generating them in Move code, and redirecting the call to the right function.
        let returned_bytes = session
            .execute_loaded_function(func, args, gas_meter)?
            .return_values
            .into_iter()
            .map(|(bytes, _)| bytes)
            .collect::<Vec<_>>();
        Ok(returned_bytes)
    }
}

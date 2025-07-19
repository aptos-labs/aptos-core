// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::AptosVM;
use aptos_framework::natives::code::PublishRequest;
use aptos_gas_algebra::Fee;
use aptos_gas_meter::AptosGasMeter;
use aptos_types::{
    fee_statement::FeeStatement,
    transaction::{ExecutionStatus, ModuleBundle},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage, output::VMOutput,
};
use move_binary_format::{compatibility::Compatibility, errors::VMResult, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    vm_status::VMStatus,
};
use move_vm_runtime::{
    module_traversal::TraversalContext, move_vm::SerializedReturnValues, LoadedFunction,
    ModuleStorage,
};
use move_vm_types::gas::GasMeter;
use std::borrow::Borrow;

/// Represents any session that can be used by Aptos VM to execute Move functions.
pub trait Session {
    /// Executes Move function (ignoring its visibility), specified by its name, with the provided
    /// arguments.
    fn execute_function_bypass_visibility(
        &mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues>;

    /// Executes already loaded Move function with the provided arguments.
    fn execute_loaded_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues>;

    /// Extracts the publish request made in the native context of execution. Returns [None] if
    /// it does not exist.
    fn extract_publish_request(&mut self) -> Option<PublishRequest>;

    /// Marks the randomness native context as unbiasable.
    fn mark_unbiasable(&mut self);
}

/// A session used by Aptos VM when executing a user transaction. Session is split into multiple
/// stages, including but not limited to:
///   1. Prologue: runs validation logic before executing user payload.
///   2. User: runs specified user payload, e.g., an entry function, a script, etc.
///   3. Epilogue: post-processing for transaction to charge gas.
///
/// For each stage, the caller must indicate its start and the corresponding end.
pub trait TransactionalSession<'a, DataView>: Session {
    /// Returns the base view used for data (resources, resource groups, configs, etc.).
    fn data_view(&self) -> &DataView;

    fn end_prologue_and_start_user_session(
        &mut self,
        vm: &AptosVM,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus>;

    fn end_user_session_without_publish_request(
        &mut self,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus>;

    fn end_user_session_with_publish_request(
        &mut self,
        module_storage: &impl AptosModuleStorage,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        destination: AccountAddress,
        bundle: ModuleBundle,
        modules: &[CompiledModule],
        compatability_checks: Compatibility,
    ) -> Result<(), VMStatus>;

    fn charge_change_set_and_start_success_epilogue(
        &mut self,
        vm: &AptosVM,
        gas_meter: &mut impl AptosGasMeter,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<Fee, VMStatus>;

    fn end_success_epilogue(
        &mut self,
        fee_statement: FeeStatement,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(VMStatus, VMOutput), VMStatus>;

    fn mark_multisig_payload_execution_failure_and_start_success_epilogue(&mut self, vm: &AptosVM);

    fn start_failure_epilogue_with_abort_hook(
        &mut self,
        vm: &AptosVM,
        gas_meter: &mut impl AptosGasMeter,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        traversal_context: &mut TraversalContext,
    ) -> Result<FeeStatement, VMStatus>;

    fn end_failure_epilogue(
        &mut self,
        fee_statement: FeeStatement,
        status: ExecutionStatus,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<VMOutput, VMStatus>;
}

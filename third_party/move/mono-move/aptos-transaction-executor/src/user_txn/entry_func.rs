// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Running an entry-function payload.

use super::args::{check_callable_signature, check_no_return_values, place_user_txn_args};
use crate::errors::{InvalidArguments, MoveExecutionFailure};
use mono_move_core::{types::InternedTypeList, Interner, PreparedModule};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{InterpreterContext, RuntimeStatus};
use move_binary_format::{access::ModuleAccess, file_format::FunctionDefinitionIndex};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Checks that a user transaction may call the given function based on info from its definition.
/// - It must not be a native.
/// - It must be an entry function.
/// - It must not return values.
fn check_callable_definition(
    module: &PreparedModule,
    def_idx: FunctionDefinitionIndex,
) -> Result<(), InvalidArguments> {
    let def = module.function_def_at(def_idx);
    if def.is_native() {
        return Err(InvalidArguments::NativeEntryFunction);
    }
    if !def.is_entry {
        return Err(InvalidArguments::NotEntryFunction);
    }
    check_no_return_values(module, def_idx)
}

/// Runs the transaction's entry function, metered against the transaction's gas budget.
pub(crate) fn call_entry_function<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    ty_args: InternedTypeList,
    sender: &AccountAddress,
    secondary_signers: &[AccountAddress],
    args: &[Vec<u8>],
) -> Result<RuntimeStatus, MoveExecutionFailure> {
    let module_id = guard.module_id_of(address, module_name);
    let function_name = guard.identifier_of(function_name);
    let module = interp
        .load_module(module_id)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    // A function the module does not define is reported by the loader below.
    if let Some(def_idx) = module.function_def_idx(function_name) {
        check_callable_definition(&module.ir().module, def_idx)
            .map_err(MoveExecutionFailure::InvalidArguments)?;
    }
    // TODO(completeness): AptosVM marks the session unbiasable when a friend
    // or private entry function carries the `#[randomness]` annotation.
    let func = interp
        .load_function(module_id, function_name, ty_args)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    let num_signer_params = check_callable_signature(guard, interp, &func.param_tys)?;
    let mut call = interp
        .build_call(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    place_user_txn_args(
        &mut call,
        num_signer_params,
        sender,
        secondary_signers,
        args,
    )?;
    call.run().map_err(MoveExecutionFailure::RuntimeError)
}

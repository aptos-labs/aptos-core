// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Running an entry-function payload.

use super::{
    arg_check::{check_arg_values, check_param_types},
    args::{check_arg_counts, leading_signer_params, place_user_txn_args},
};
use crate::{
    calls::resolve_function_by_name,
    errors::{InvalidArguments, MoveExecutionFailure},
};
use mono_move_core::{types::InternedTypeList, Function, PreparedModule};
use mono_move_global_context::ExecutionGuard;
use mono_move_loader::LoaderError;
use mono_move_runtime::{InterpreterContext, RuntimeStatus};
use move_binary_format::access::ModuleAccess;
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Checks that `func` is allowed to be called by a user transaction, returning
/// the number of leading signer parameters.
/// - It must be an entry function.
/// - It must not return values.
/// - All signers must be in leading positions.
fn check_callable_by_user_txn(
    func: &Function,
    module: &PreparedModule,
) -> Result<usize, InvalidArguments> {
    let def = module.function_def_at(func.def_idx);
    if !def.is_entry {
        return Err(InvalidArguments::NotEntryFunction);
    }
    let handle = module.function_handle_at(def.function);
    if !module.interned_types_at(handle.return_).is_empty() {
        return Err(InvalidArguments::ReturnsValues);
    }
    leading_signer_params(&func.param_tys)
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
    let func =
        match resolve_function_by_name(guard, interp, address, module_name, function_name, ty_args)
        {
            Ok(func) => func,
            // A native never lowers, so it surfaces as a load failure; AptosVM
            // loads it and then refuses to run it.
            Err(err)
                if matches!(
                    err.downcast_ref::<LoaderError>(),
                    Some(LoaderError::NativeFunctionNotLoadable { .. })
                ) =>
            {
                return Err(MoveExecutionFailure::InvalidArguments(
                    InvalidArguments::NativeEntryFunction,
                ));
            },
            Err(err) => return Err(MoveExecutionFailure::RuntimeError(err)),
        };
    // TODO(completeness): AptosVM marks the session unbiasable when a friend
    // or private entry function carries the `#[randomness]` annotation.
    let module = interp
        .module_of(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    let signer_params =
        check_callable_by_user_txn(func, module).map_err(MoveExecutionFailure::InvalidArguments)?;
    check_param_types(guard, interp, &func.param_tys[signer_params..])?;
    check_arg_counts(&func.param_tys, signer_params, secondary_signers, args)
        .map_err(MoveExecutionFailure::InvalidArguments)?;
    check_arg_values(guard, interp, &func.param_tys[signer_params..], args)?;
    let mut call = interp
        .build_call(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    place_user_txn_args(&mut call, signer_params, sender, secondary_signers, args)?;
    call.run().map_err(MoveExecutionFailure::RuntimeError)
}

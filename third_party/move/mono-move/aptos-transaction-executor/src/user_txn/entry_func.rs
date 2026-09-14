// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Running an entry-function payload: a generated script deserializes the
//! arguments in Move and calls the function.

use super::{
    args::{check_arg_counts, leading_signer_params},
    trampoline::trampoline_for,
    txn_arg,
};
use crate::{
    calls::resolve_function_by_name,
    errors::{InvalidArguments, MoveExecutionFailure},
};
use mono_move_core::{types::InternedTypeList, Function, PreparedModule, VMInternalError};
use mono_move_global_context::ExecutionGuard;
use mono_move_loader::LoaderError;
use mono_move_runtime::{InterpreterContext, RuntimeStatus};
use move_binary_format::access::ModuleAccess;
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, vm_status::AbortLocation,
};
use specializer::NotATransactionArgument;

/// Checks that `func` is allowed to be called by a user transaction, returning
/// the number of leading signer parameters.
/// - It must be an entry function.
/// - It must not return values.
/// - All signers must be in leading positions.
///
/// Whether the other parameters have types a transaction may supply is
/// decided by loading their deserializers.
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

/// Runs the transaction's entry function, metered against the transaction's
/// gas budget.
//
// TODO(completeness): public structs and enums, and signed integers, have no
// deserializer yet and are refused.
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
    check_arg_counts(&func.param_tys, signer_params, secondary_signers, args)
        .map_err(MoveExecutionFailure::InvalidArguments)?;
    // Refuse a parameter type no transaction may supply before running
    // anything, as AptosVM does.
    for &ty in &func.param_tys[signer_params..] {
        txn_arg::load_deserializer(guard, interp, ty).map_err(construction_failure)?;
    }

    let trampoline = trampoline_for(module, func.def_idx);
    let main = interp
        .load_trusted_script(&trampoline, ty_args)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    let mut call = interp
        .build_call(main)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    // The sender fills the first signer parameter, secondary signers the
    // rest. Placing a signer can only fail on a bug.
    for signer in std::iter::once(sender)
        .chain(secondary_signers)
        .take(signer_params)
    {
        call.signer(signer)
            .map_err(MoveExecutionFailure::RuntimeError)?;
    }
    for arg in args {
        call.arg(arg.as_slice())
            .map_err(MoveExecutionFailure::RuntimeError)?;
    }
    match call.run().map_err(construction_failure)? {
        // The argument module aborts only on bytes that do not encode the
        // parameter's type.
        RuntimeStatus::Aborted {
            location: AbortLocation::Module(module_id),
            ..
        } if txn_arg::is_txn_arg_module(&module_id) => Err(MoveExecutionFailure::InvalidArguments(
            InvalidArguments::UndecodableArgument,
        )),
        status => Ok(status),
    }
}

/// A failure while loading or running argument deserializers. A type no
/// transaction may supply surfaces as a lowering error.
fn construction_failure(err: VMInternalError) -> MoveExecutionFailure {
    if err.downcast_ref::<NotATransactionArgument>().is_some() {
        MoveExecutionFailure::InvalidArguments(InvalidArguments::DisallowedParameterType)
    } else {
        MoveExecutionFailure::RuntimeError(err)
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Placing a transaction payload's wire-format arguments -- signer addresses
//! and BCS blobs -- onto an entry function's parameters.

use crate::{
    calls::resolve_function_by_name,
    errors::{InvalidArguments, MoveExecutionFailure},
};
use mono_move_core::types::{is_signer_or_signer_immut_ref, InternedTypeList};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{CallBuilder, InterpreterContext, RuntimeError, RuntimeStatus};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Fills the call in parameter order: signer parameters from the sender and
/// secondary signers, everything else from the transaction's BCS arguments.
fn place_user_txn_args<'a>(
    call: &mut CallBuilder<'a, '_>,
    sender: &'a AccountAddress,
    secondary_signers: &'a [AccountAddress],
    args: &[Vec<u8>],
) -> Result<(), MoveExecutionFailure> {
    // Like AptosVM, check the parameter shape and both counts before decoding
    // any argument: signers must lead the parameter list, and a function with
    // signer parameters requires exactly that many signers while one without
    // ignores them.
    let mut signer_params = 0;
    for (index, &ty) in call.param_tys().iter().enumerate() {
        if is_signer_or_signer_immut_ref(ty) {
            if index != signer_params {
                return Err(MoveExecutionFailure::InvalidArguments(
                    InvalidArguments::SignerAfterArgument,
                ));
            }
            signer_params += 1;
        }
    }
    if args.len() != call.param_tys().len() - signer_params {
        return Err(MoveExecutionFailure::InvalidArguments(
            InvalidArguments::ArgumentCountMismatch,
        ));
    }
    if signer_params > 0 && 1 + secondary_signers.len() != signer_params {
        return Err(MoveExecutionFailure::InvalidArguments(
            InvalidArguments::SignerCountMismatch,
        ));
    }
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
        // Only a decode failure faults the argument's bytes; anything else
        // (a non-decodable parameter type, an exhausted heap) is the VM's.
        call.arg_bcs(arg).map_err(|err| {
            if err
                .downcast_ref::<RuntimeError>()
                .is_some_and(RuntimeError::is_bcs_decode_error)
            {
                MoveExecutionFailure::InvalidArguments(InvalidArguments::UndecodableArgument)
            } else {
                MoveExecutionFailure::RuntimeError(err)
            }
        })?;
    }
    Ok(())
}

/// Runs the transaction's entry function on its wire-format arguments,
/// metered against the transaction's gas budget.
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
        resolve_function_by_name(guard, interp, address, module_name, function_name, ty_args)
            .map_err(MoveExecutionFailure::RuntimeError)?;
    let mut call = interp
        .build_call(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    place_user_txn_args(&mut call, sender, secondary_signers, args)?;
    call.run().map_err(MoveExecutionFailure::RuntimeError)
}

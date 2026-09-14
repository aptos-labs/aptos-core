// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Placing a transaction payload's wire-format arguments -- signer addresses
//! and BCS blobs -- onto the payload's parameters.

use crate::errors::{InvalidArguments, MoveExecutionFailure};
use mono_move_core::types::{is_signer_or_signer_immut_ref, InternedType};
use mono_move_runtime::{CallBuilder, RuntimeError};
use move_core_types::account_address::AccountAddress;

/// Counts the leading signer parameters, rejecting a signer that follows a
/// non-signer parameter.
pub(super) fn leading_signer_params(param_tys: &[InternedType]) -> Result<usize, InvalidArguments> {
    let signer_params = param_tys
        .iter()
        .take_while(|&&ty| is_signer_or_signer_immut_ref(ty))
        .count();
    if param_tys[signer_params..]
        .iter()
        .any(|&ty| is_signer_or_signer_immut_ref(ty))
    {
        return Err(InvalidArguments::SignerAfterArgument);
    }
    Ok(signer_params)
}

/// Fills the call in parameter order: the `signer_params` leading signer
/// parameters from the sender and secondary signers, everything else from the
/// transaction's BCS arguments.
pub(super) fn place_user_txn_args<'a>(
    call: &mut CallBuilder<'a, '_>,
    signer_params: usize,
    sender: &'a AccountAddress,
    secondary_signers: &'a [AccountAddress],
    args: &[Vec<u8>],
) -> Result<(), MoveExecutionFailure> {
    // Like AptosVM, check both counts before decoding any argument: a function
    // with signer parameters requires exactly that many signers, while one
    // without ignores them.
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

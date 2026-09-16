// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lazy creation of the sender's account.

use super::metadata::TxnMetadata;
use crate::{
    calls::resolve_function_by_name,
    errors::{call_result, invariant_violation, MoveExecutionFailure},
};
use aptos_types::{account_config::AccountResource, transaction::ReplayProtector};
use mono_move_core::{intern_struct_tag, Interner, VMInternalError};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::InterpreterContext;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, move_resource::MoveStructType,
};

const ACCOUNT: &IdentStr = ident_str!("account");
const CREATE_ACCOUNT_IF_DOES_NOT_EXIST: &IdentStr = ident_str!("create_account_if_does_not_exist");

/// Whether the sender's account must be created before the payload runs: this
/// is the sender's first transaction (sequence number 0) and no `Account`
/// resource is stored at its address.
pub(crate) fn needs_account_creation<'a>(
    interp: &mut InterpreterContext<'a>,
    guard: &ExecutionGuard<'a>,
    txn_data: &TxnMetadata,
) -> Result<bool, VMInternalError> {
    if txn_data.replay_protector != ReplayProtector::SequenceNumber(0) {
        return Ok(false);
    }
    let account_ty = intern_struct_tag(&AccountResource::struct_tag(), guard)
        .map_err(|e| invariant_violation(format!("the account type does not intern: {e:#}")))?;
    // Unmetered, as AptosVM reads it.
    let exists = interp.unmetered(|interp| interp.resource_exists(txn_data.sender, account_ty))?;
    Ok(!exists)
}

/// Creates the sender's account, metered against the transaction's gas budget.
pub(crate) fn create_account<'a>(
    interp: &mut InterpreterContext<'a>,
    guard: &ExecutionGuard<'a>,
    sender: &AccountAddress,
) -> Result<(), MoveExecutionFailure> {
    let func = resolve_function_by_name(
        guard,
        interp,
        &AccountAddress::ONE,
        ACCOUNT,
        CREATE_ACCOUNT_IF_DOES_NOT_EXIST,
        guard.type_list_of(&[]),
    )
    .map_err(MoveExecutionFailure::RuntimeError)?;
    let mut call = interp
        .build_call(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    call.arg(sender)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    let status = call.run().map_err(MoveExecutionFailure::RuntimeError)?;
    call_result(status)
}

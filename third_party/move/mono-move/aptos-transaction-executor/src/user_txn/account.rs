// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lazy creation of the sender's account.

use super::metadata::TxnMetadata;
use crate::errors::{call_result, MoveExecutionFailure};
use aptos_types::transaction::ReplayProtector;
use mono_move_core::{types::EMPTY_TYPE_LIST, FrameworkSymbols, VMInternalError};
use mono_move_runtime::{CompletedCall, InterpreterContext};
use move_core_types::account_address::AccountAddress;

/// Whether the sender's account must be created before the payload runs.
/// - This is the sender's first transaction (sequence number 0).
/// - No `Account` resource is stored at its address.
pub(crate) fn needs_account_creation<'a>(
    interp: &mut InterpreterContext<'a>,
    symbols: &FrameworkSymbols,
    txn_data: &TxnMetadata,
) -> Result<bool, VMInternalError> {
    if txn_data.replay_protector != ReplayProtector::SequenceNumber(0) {
        return Ok(false);
    }
    // Unmetered, as AptosVM reads it.
    let exists = interp
        .unmetered(|interp| interp.resource_exists(txn_data.sender, symbols.account_resource))?;
    Ok(!exists)
}

/// Creates the sender's account, metered against the transaction's gas budget.
pub(crate) fn create_account<'a>(
    interp: &mut InterpreterContext<'a>,
    symbols: &FrameworkSymbols,
    sender: &AccountAddress,
) -> Result<(), MoveExecutionFailure> {
    let func = interp
        .load_function(
            symbols.account,
            symbols.create_account_if_does_not_exist,
            EMPTY_TYPE_LIST,
        )
        .map_err(MoveExecutionFailure::RuntimeError)?;
    let mut call = interp
        .build_call(func)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    call.arg(sender)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    let status = call
        .run()
        .map(CompletedCall::into_status)
        .map_err(|err| MoveExecutionFailure::RuntimeError(err.into_error()))?;
    call_result(status)
}

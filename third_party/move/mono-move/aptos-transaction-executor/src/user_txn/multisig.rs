// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Running a multisig transaction: the framework checks that the owners
//! approved the payload, the payload runs as the multisig account, and the
//! account records the outcome.

use super::{entry_func::call_entry_function, metadata::TxnMetadata, script::run_script};
use crate::{
    calls::{call_system_function, call_system_function_unmetered},
    errors::{
        call_result, invariant_violation, DiscardReason, ExecutionStage, ExecutionStatus,
        MoveExecutionFailure,
    },
    materialize::executed_vm_status,
};
use aptos_types::{
    chain_id::ChainId,
    transaction::{ExecutionError, MultisigTransactionPayload, TransactionExecutableRef},
};
use mono_move_core::{intern_type_tag, types::InternedTypeList, Interner};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::InterpreterContext;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::TypeTag,
};

const MULTISIG_ACCOUNT: &IdentStr = ident_str!("multisig_account");
const VALIDATE_MULTISIG_TRANSACTION: &IdentStr = ident_str!("validate_multisig_transaction");
const GET_NEXT_TRANSACTION_PAYLOAD: &IdentStr = ident_str!("get_next_transaction_payload");
const SUCCESSFUL_TRANSACTION_EXECUTION_CLEANUP: &IdentStr =
    ident_str!("successful_transaction_execution_cleanup");
const FAILED_TRANSACTION_EXECUTION_CLEANUP: &IdentStr =
    ident_str!("failed_transaction_execution_cleanup");

/// The payload a multisig transaction provides, encoded as the framework
/// checks it against the stored transaction; empty when it relies on the
/// stored payload.
pub(crate) fn provided_payload_bytes(
    executable: TransactionExecutableRef<'_>,
) -> Result<Vec<u8>, DiscardReason> {
    let payload = match executable {
        TransactionExecutableRef::EntryFunction(entry) => {
            MultisigTransactionPayload::EntryFunction(entry.clone())
        },
        TransactionExecutableRef::Script(script) => {
            MultisigTransactionPayload::Script(script.clone())
        },
        TransactionExecutableRef::Empty => return Ok(vec![]),
        TransactionExecutableRef::Encrypted => {
            return Err(DiscardReason::Unsupported("encrypted multisig payloads"))
        },
    };
    bcs::to_bytes(&payload).map_err(|e| {
        DiscardReason::InvariantViolation(format!("the multisig payload does not serialize: {e}"))
    })
}

/// Checks with the framework that the sender may execute the multisig
/// account's next transaction with `provided_payload`.
pub(crate) fn run_multisig_prologue<'a>(
    interp: &mut InterpreterContext<'a>,
    guard: &ExecutionGuard<'a>,
    txn_data: &TxnMetadata,
    multisig_address: AccountAddress,
    provided_payload: &[u8],
) -> Result<(), MoveExecutionFailure> {
    let status = call_system_function_unmetered(
        guard,
        interp,
        &AccountAddress::ONE,
        MULTISIG_ACCOUNT,
        VALIDATE_MULTISIG_TRANSACTION,
        guard.type_list_of(&[]),
        &[txn_data.sender],
        |call| {
            call.arg(&multisig_address)?;
            call.arg(&provided_payload.to_vec())
        },
    )
    .map_err(MoveExecutionFailure::RuntimeError)?;
    call_result(status)
}

/// Runs the multisig account's next transaction as the account and has the
/// account record the outcome. A payload that fails is rolled back and
/// recorded as failed; the transaction itself still succeeds. Returns an error
/// only when the payload could not be obtained or the account could not
/// record the outcome.
pub(crate) fn execute_multisig_payload<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    chain_id: ChainId,
    txn_data: &TxnMetadata,
    multisig_address: AccountAddress,
    provided_payload: &[u8],
) -> Result<(), MoveExecutionFailure> {
    let payload_bytes =
        next_transaction_payload(guard, interp, multisig_address, provided_payload)?;
    let payload = bcs::from_bytes::<MultisigTransactionPayload>(&payload_bytes)
        .map_err(|_| MoveExecutionFailure::UndecodableMultisigPayload)?;

    match run_payload(guard, interp, chain_id, multisig_address, &payload) {
        Ok(()) => call_system_function_unmetered(
            guard,
            interp,
            &AccountAddress::ONE,
            MULTISIG_ACCOUNT,
            SUCCESSFUL_TRANSACTION_EXECUTION_CLEANUP,
            guard.type_list_of(&[]),
            &[],
            |call| {
                call.arg(&txn_data.sender)?;
                call.arg(&multisig_address)?;
                call.arg(&payload_bytes)
            },
        ),
        Err(failure) => {
            // Drop the payload's effects but keep the prologue's, then let the
            // account record why the payload failed.
            interp
                .rollback(1)
                .and_then(|()| interp.checkpoint())
                .map_err(MoveExecutionFailure::RuntimeError)?;
            let error = execution_error(failure)?;
            let error_bytes = bcs::to_bytes(&error).map_err(|e| {
                MoveExecutionFailure::RuntimeError(invariant_violation(format!(
                    "the execution error does not serialize: {e}"
                )))
            })?;
            call_system_function_unmetered(
                guard,
                interp,
                &AccountAddress::ONE,
                MULTISIG_ACCOUNT,
                FAILED_TRANSACTION_EXECUTION_CLEANUP,
                guard.type_list_of(&[]),
                &[],
                |call| {
                    call.arg(&txn_data.sender)?;
                    call.arg(&multisig_address)?;
                    call.arg(&payload_bytes)?;
                    call.arg_bcs(&error_bytes)
                },
            )
        },
    }
    .map_err(MoveExecutionFailure::RuntimeError)
    .and_then(call_result)
}

/// The payload the multisig account executes next: the stored one, or
/// `provided_payload` if the transaction was created with a hash only.
fn next_transaction_payload<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    multisig_address: AccountAddress,
    provided_payload: &[u8],
) -> Result<Vec<u8>, MoveExecutionFailure> {
    let status = call_system_function(
        guard,
        interp,
        &AccountAddress::ONE,
        MULTISIG_ACCOUNT,
        GET_NEXT_TRANSACTION_PAYLOAD,
        guard.type_list_of(&[]),
        &[],
        |call| {
            call.arg(&multisig_address)?;
            call.arg(&provided_payload.to_vec())
        },
    )
    .map_err(MoveExecutionFailure::RuntimeError)?;
    call_result(status)?;

    let bytes_ty = intern_type_tag(&TypeTag::Vector(Box::new(TypeTag::U8)), guard)
        .map_err(|e| MoveExecutionFailure::RuntimeError(invariant_violation(format!("{e:#}"))))?;
    let result = interp
        .serialize_root_result(bytes_ty)
        .map_err(MoveExecutionFailure::RuntimeError)?;
    bcs::from_bytes::<Vec<u8>>(&result)
        .map_err(|_| MoveExecutionFailure::UndecodableMultisigPayload)
}

/// Runs `payload` as the multisig account, metered.
fn run_payload<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    chain_id: ChainId,
    multisig_address: AccountAddress,
    payload: &MultisigTransactionPayload,
) -> Result<(), MoveExecutionFailure> {
    let ty_args = match payload {
        MultisigTransactionPayload::EntryFunction(entry) => entry.ty_args(),
        MultisigTransactionPayload::Script(script) => script.ty_args(),
    };
    let ty_args = intern_ty_args(guard, ty_args)?;
    let status = match payload {
        MultisigTransactionPayload::EntryFunction(entry) => call_entry_function(
            guard,
            interp,
            &entry.module().address,
            entry.module().name(),
            entry.function(),
            ty_args,
            &multisig_address,
            &[],
            entry.args(),
        )?,
        MultisigTransactionPayload::Script(script) => run_script(
            guard,
            interp,
            chain_id,
            script.code(),
            ty_args,
            &multisig_address,
            &[],
            script.args(),
        )?,
    };
    call_result(status)
}

/// Interns the type arguments of a stored payload.
//
// TODO(security): these type arguments come from chain state a user wrote, so
// interning them can pollute the global caches. Needs a bound.
fn intern_ty_args(
    guard: &ExecutionGuard<'_>,
    ty_args: &[TypeTag],
) -> Result<InternedTypeList, MoveExecutionFailure> {
    let interned = ty_args
        .iter()
        .map(|tag| intern_type_tag(tag, guard))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| MoveExecutionFailure::UndecodableMultisigPayload)?;
    Ok(guard.type_list_of(&interned))
}

/// The payload's failure as the framework records it.
//
// TODO(cleanup): the execution path reaches into `materialize` for the status
// projection; decide where the projection belongs.
fn execution_error(failure: MoveExecutionFailure) -> Result<ExecutionError, MoveExecutionFailure> {
    let status = executed_vm_status(&ExecutionStatus::Failure {
        stage: ExecutionStage::Payload,
        failure,
    });
    ExecutionError::try_from(status).map_err(|e| {
        MoveExecutionFailure::RuntimeError(invariant_violation(format!(
            "the multisig payload's failure has no execution error: {e}"
        )))
    })
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Unmetered calls into framework system functions with Rust-built arguments.
//! Today: the transaction prologue and epilogue
//! (`0x1::transaction_validation`); the block prologue and epilogue will live
//! here too. Only the versioned validation path is supported; the many legacy
//! prologue/epilogue variants are intentionally not ported.

use crate::{
    calls::{call_function, CallStatus},
    errors::MoveExecutionFailure,
    metadata::TxnMetadata,
};
use aptos_types::{
    fee_statement::FeeStatement,
    transaction::{EpilogueArgs, PrologueArgs},
};
use mono_move_core::{types::InternedTypeList, Interner, VMInternalError};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::InterpreterContext;
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};
use serde::Serialize;

const TRANSACTION_VALIDATION: &IdentStr = ident_str!("transaction_validation");
const VERSIONED_PROLOGUE: &IdentStr = ident_str!("versioned_prologue");
const VERSIONED_EPILOGUE: &IdentStr = ident_str!("versioned_epilogue");

/// The sender and fee-payer signer buffers every validation call takes.
pub(crate) fn validation_signers(txn_data: &TxnMetadata) -> [[u8; AccountAddress::LENGTH]; 2] {
    let fee_payer = txn_data.fee_payer.unwrap_or(txn_data.sender);
    [txn_data.sender.into_bytes(), fee_payer.into_bytes()]
}

/// Like `call_function`, but unmetered: system code never consumes the
/// transaction's gas budget, including its module loads.
fn call_system_function(
    guard: &ExecutionGuard<'_>,
    interp: &mut InterpreterContext<'_>,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    ty_args: InternedTypeList,
    signer_bufs: &[[u8; AccountAddress::LENGTH]],
    args: &[Vec<u8>],
) -> Result<CallStatus, VMInternalError> {
    interp.unmetered(|interp| {
        call_function(
            guard,
            interp,
            address,
            module_name,
            function_name,
            ty_args,
            signer_bufs,
            args,
        )
    })
}

/// Calls `0x1::transaction_validation::<function>(signers…, args)`,
/// unmetered.
fn call_validation_function(
    guard: &ExecutionGuard<'_>,
    interp: &mut InterpreterContext<'_>,
    function: &IdentStr,
    signers: &[[u8; AccountAddress::LENGTH]; 2],
    args: &impl Serialize,
) -> Result<CallStatus, VMInternalError> {
    let args_blob = bcs::to_bytes(args).expect("validation args serialize");
    call_system_function(
        guard,
        interp,
        &AccountAddress::ONE,
        TRANSACTION_VALIDATION,
        function,
        guard.type_list_of(&[]),
        signers,
        &[args_blob],
    )
}

pub(crate) fn run_prologue(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    signers: &[[u8; AccountAddress::LENGTH]; 2],
    txn_data: &TxnMetadata,
) -> Result<(), MoveExecutionFailure> {
    let args = PrologueArgs::V1 {
        needs_fee_payer_auth_check: txn_data.fee_payer.is_some(),
        txn_sender_public_key: txn_data.sender_auth_key.clone(),
        fee_payer_public_key_hash: txn_data.fee_payer_auth_key.clone(),
        replay_protector: txn_data.replay_protector,
        secondary_signer_addresses: txn_data.secondary_signers.clone(),
        secondary_signer_public_key_hashes: txn_data.secondary_auth_keys.clone(),
        txn_gas_price: txn_data.gas_unit_price,
        txn_max_gas_units: txn_data.max_gas_amount,
        txn_expiration_time: txn_data.expiration_timestamp_secs,
        chain_id: txn_data.chain_id,
        is_simulation: false,
        // TODO(completeness): transaction limits requests (staking multipliers).
        txn_limits_request: None,
    };
    let status = call_validation_function(guard, interp, VERSIONED_PROLOGUE, signers, &args)
        .map_err(MoveExecutionFailure::RuntimeError)?;

    match status {
        CallStatus::Success => Ok(()),
        CallStatus::Abort {
            code,
            message,
            location,
        } => Err(MoveExecutionFailure::Abort {
            code,
            message,
            location,
        }),
    }
}

pub(crate) fn run_epilogue(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    signers: &[[u8; AccountAddress::LENGTH]; 2],
    txn_data: &TxnMetadata,
    fee_statement: FeeStatement,
    gas_units_remaining: u64,
) -> Result<(), MoveExecutionFailure> {
    let args = EpilogueArgs::V1 {
        fee_statement,
        txn_gas_price: txn_data.gas_unit_price,
        txn_max_gas_units: txn_data.max_gas_amount,
        gas_units_remaining,
        is_simulation: false,
        is_orderless_txn: txn_data.is_orderless(),
    };
    let status = call_validation_function(guard, interp, VERSIONED_EPILOGUE, signers, &args)
        .map_err(MoveExecutionFailure::RuntimeError)?;

    match status {
        CallStatus::Success => Ok(()),
        CallStatus::Abort {
            code,
            message,
            location,
        } => Err(MoveExecutionFailure::Abort {
            code,
            message,
            location,
        }),
    }
}

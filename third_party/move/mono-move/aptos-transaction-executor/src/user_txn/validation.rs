// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Unmetered calls into the transaction prologue and epilogue
//! (`0x1::transaction_validation`), with Rust-built arguments. Only the
//! versioned validation path is supported; the many legacy prologue/epilogue
//! variants are intentionally not ported.

use super::metadata::TxnMetadata;
use crate::{
    calls::call_system_function_unmetered,
    errors::{call_result, MoveExecutionFailure},
};
use aptos_types::{
    fee_statement::FeeStatement,
    transaction::{EpilogueArgs, PrologueArgs},
};
use mono_move_core::{Interner, VMInternalError};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{InterpreterContext, RuntimeStatus};
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};
use move_value_view::MoveValueView;

const TRANSACTION_VALIDATION: &IdentStr = ident_str!("transaction_validation");
const VERSIONED_PROLOGUE: &IdentStr = ident_str!("versioned_prologue");
const VERSIONED_EPILOGUE: &IdentStr = ident_str!("versioned_epilogue");

/// The signers the prologue and epilogue take.
pub(crate) struct ValidationSigners {
    sender: AccountAddress,
    fee_payer: AccountAddress,
}

impl ValidationSigners {
    pub(crate) fn new(txn_data: &TxnMetadata) -> Self {
        Self {
            sender: txn_data.sender,
            fee_payer: txn_data.fee_payer.unwrap_or(txn_data.sender),
        }
    }
}

/// Calls `0x1::transaction_validation::<function>(sender, fee_payer, args)`.
fn call_validation_function_unmetered<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    function: &IdentStr,
    signers: &ValidationSigners,
    args: &impl MoveValueView,
) -> Result<RuntimeStatus, VMInternalError> {
    call_system_function_unmetered(
        guard,
        interp,
        &AccountAddress::ONE,
        TRANSACTION_VALIDATION,
        function,
        guard.type_list_of(&[]),
        &[signers.sender, signers.fee_payer],
        |call| call.arg(args),
    )
}

pub(crate) fn run_prologue<'a>(
    interp: &mut InterpreterContext<'a>,
    guard: &ExecutionGuard<'a>,
    signers: &ValidationSigners,
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
    let status =
        call_validation_function_unmetered(guard, interp, VERSIONED_PROLOGUE, signers, &args)
            .map_err(MoveExecutionFailure::RuntimeError)?;
    call_result(status)
}

pub(crate) fn run_epilogue<'a>(
    interp: &mut InterpreterContext<'a>,
    guard: &ExecutionGuard<'a>,
    signers: &ValidationSigners,
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
    let status =
        call_validation_function_unmetered(guard, interp, VERSIONED_EPILOGUE, signers, &args)
            .map_err(MoveExecutionFailure::RuntimeError)?;
    call_result(status)
}

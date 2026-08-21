// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Unmetered calls into the transaction prologue and epilogue
//! (`0x1::transaction_validation`), with Rust-built arguments. Only the
//! versioned validation path is supported; the many legacy prologue/epilogue
//! variants are intentionally not ported.

use super::metadata::TxnMetadata;
use crate::{
    calls::{call_system_function_unmetered, into_result, invariant_violation},
    errors::MoveExecutionFailure,
};
use anyhow::anyhow;
use aptos_types::{
    fee_statement::FeeStatement,
    transaction::{EpilogueArgs, PrologueArgs},
};
use mono_move_core::{Interner, VMInternalError};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{InterpreterContext, RuntimeStatus};
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};
use serde::Serialize;

const TRANSACTION_VALIDATION: &IdentStr = ident_str!("transaction_validation");
const VERSIONED_PROLOGUE: &IdentStr = ident_str!("versioned_prologue");
const VERSIONED_EPILOGUE: &IdentStr = ident_str!("versioned_epilogue");

/// The signer buffers a Move call receives, in parameter order.
pub(crate) struct TxnSigners(Vec<[u8; AccountAddress::LENGTH]>);

impl TxnSigners {
    /// Sender and fee payer, as the prologue and epilogue take them.
    pub(crate) fn for_validation(txn_data: &TxnMetadata) -> Self {
        let fee_payer = txn_data.fee_payer.unwrap_or(txn_data.sender);
        Self(vec![txn_data.sender.into_bytes(), fee_payer.into_bytes()])
    }

    /// Sender and any secondary signers, as an entry function takes them.
    pub(crate) fn for_payload(txn_data: &TxnMetadata) -> Self {
        let mut signers = vec![txn_data.sender.into_bytes()];
        signers.extend(txn_data.secondary_signers.iter().map(|s| s.into_bytes()));
        Self(signers)
    }

    pub(crate) fn as_slice(&self) -> &[[u8; AccountAddress::LENGTH]] {
        &self.0
    }
}

/// Calls `0x1::transaction_validation::<function>(signers…, args)`.
fn call_validation_function_unmetered(
    guard: &ExecutionGuard<'_>,
    interp: &mut InterpreterContext<'_>,
    function: &IdentStr,
    signers: &TxnSigners,
    args: &impl Serialize,
) -> Result<RuntimeStatus, VMInternalError> {
    let args_blob = bcs::to_bytes(args)
        .map_err(|e| invariant_violation(anyhow!("validation args do not serialize: {e}")))?;
    call_system_function_unmetered(
        guard,
        interp,
        &AccountAddress::ONE,
        TRANSACTION_VALIDATION,
        function,
        guard.type_list_of(&[]),
        signers.as_slice(),
        &[args_blob],
    )
}

pub(crate) fn run_prologue(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    signers: &TxnSigners,
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
    into_result(status)
}

pub(crate) fn run_epilogue(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    signers: &TxnSigners,
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
    into_result(status)
}

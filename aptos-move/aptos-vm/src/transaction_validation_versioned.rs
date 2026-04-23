// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    aptos_vm::SerializedSigners,
    errors::convert_prologue_error,
    move_vm_ext::{AptosMoveResolver, SessionExt},
    system_module_names::{
        TRANSACTION_VALIDATION_MODULE, VERSIONED_EPILOGUE_NAME, VERSIONED_PROLOGUE_NAME,
    },
    testing::{maybe_raise_injected_error, InjectedError},
    transaction_metadata::TransactionMetadata,
};
use aptos_gas_algebra::Gas;
use aptos_types::{
    fee_statement::FeeStatement,
    transaction::{ReplayProtector, TxnLimitsRequest, UserTxnLimitsRequest},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use move_binary_format::errors::VMResult;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::{
    logging::expect_no_verification_errors, module_traversal::TraversalContext, ModuleStorage,
};
use move_vm_types::gas::UnmeteredGasMeter;
use serde::Serialize;

/// Mirrors Move enum in `transaction_validation.move` and needs to have the
/// same BCS serialization.
#[derive(Serialize)]
enum PrologueArgs {
    V1 {
        txn_sender_public_key: Option<Vec<u8>>,
        fee_payer_public_key_hash: Option<Vec<u8>>,
        replay_protector: ReplayProtector,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signer_public_key_hashes: Vec<Option<Vec<u8>>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        is_simulation: bool,
        txn_limits_request: Option<UserTxnLimitsRequest>,
    },
}

/// Builder that collects prologue arguments and selects the appropriate enum
/// variant to build.
pub(crate) struct PrologueBuilder {
    txn_sender_public_key: Option<Vec<u8>>,
    fee_payer_public_key_hash: Option<Vec<u8>>,
    replay_protector: ReplayProtector,
    secondary_signer_addresses: Vec<AccountAddress>,
    secondary_signer_public_key_hashes: Vec<Option<Vec<u8>>>,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    chain_id: u8,
    is_simulation: bool,
    txn_limits_request: Option<UserTxnLimitsRequest>,
}

impl PrologueBuilder {
    pub fn new(txn_data: &TransactionMetadata, is_simulation: bool) -> Self {
        Self {
            txn_sender_public_key: txn_data.authentication_proof().optional_auth_key(),
            fee_payer_public_key_hash: txn_data
                .fee_payer_authentication_proof
                .as_ref()
                .and_then(|proof| proof.optional_auth_key()),
            replay_protector: txn_data.replay_protector(),
            secondary_signer_addresses: txn_data.secondary_signers(),
            secondary_signer_public_key_hashes: txn_data
                .secondary_authentication_proofs
                .iter()
                .map(|proof| proof.optional_auth_key())
                .collect(),
            txn_gas_price: txn_data.gas_unit_price().into(),
            txn_max_gas_units: txn_data.max_gas_amount().into(),
            txn_expiration_time: txn_data.expiration_timestamp_secs(),
            chain_id: txn_data.chain_id().id(),
            is_simulation,
            txn_limits_request: txn_data.txn_limits.as_ref().and_then(|v| match v {
                TxnLimitsRequest::ApprovedGovernanceScript => None,
                TxnLimitsRequest::Staking(req) => Some(req.clone()),
            }),
        }
    }

    /// Selects the highest supported variant based on feature flags and BCS-serializes it.
    /// Currently only V1 exists.
    pub fn build(self) -> Vec<u8> {
        let args = PrologueArgs::V1 {
            txn_sender_public_key: self.txn_sender_public_key,
            fee_payer_public_key_hash: self.fee_payer_public_key_hash,
            replay_protector: self.replay_protector,
            secondary_signer_addresses: self.secondary_signer_addresses,
            secondary_signer_public_key_hashes: self.secondary_signer_public_key_hashes,
            txn_gas_price: self.txn_gas_price,
            txn_max_gas_units: self.txn_max_gas_units,
            txn_expiration_time: self.txn_expiration_time,
            chain_id: self.chain_id,
            is_simulation: self.is_simulation,
            txn_limits_request: self.txn_limits_request,
        };
        bcs::to_bytes(&args).expect("Failed to serialize prologue arguments")
    }
}

pub(crate) fn run_prologue(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    serialized_signers: &SerializedSigners,
    txn_data: &TransactionMetadata,
    log_context: &AdapterLogSchema,
    traversal_context: &mut TraversalContext,
    is_simulation: bool,
) -> Result<(), move_core_types::vm_status::VMStatus> {
    let builder = PrologueBuilder::new(txn_data, is_simulation);
    let serialized_args = vec![
        serialized_signers.sender(),
        serialized_signers
            .fee_payer()
            .unwrap_or(serialized_signers.sender()),
        builder.build(),
    ];
    session
        .execute_function_bypass_visibility(
            &TRANSACTION_VALIDATION_MODULE,
            VERSIONED_PROLOGUE_NAME,
            vec![],
            serialized_args,
            &mut UnmeteredGasMeter,
            traversal_context,
            module_storage,
        )
        .map(|_return_vals| ())
        .map_err(expect_no_verification_errors)
        .or_else(|err| convert_prologue_error(err, log_context))
}

/// Mirrors Move enum in `transaction_validation.move` and needs to have the
/// same BCS serialization.
#[derive(Serialize)]
enum EpilogueArgs {
    V1 {
        fee_statement: FeeStatement,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64,
        is_simulation: bool,
        is_orderless_txn: bool,
    },
}

/// Builder that collects epilogue arguments and selects the appropriate enum
/// variant based on feature flags.
pub(crate) struct EpilogueBuilder {
    fee_statement: FeeStatement,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64,
    is_simulation: bool,
    is_orderless_txn: bool,
}

impl EpilogueBuilder {
    pub fn new(
        fee_statement: FeeStatement,
        txn_data: &TransactionMetadata,
        gas_remaining: Gas,
        is_simulation: bool,
    ) -> Self {
        Self {
            fee_statement,
            txn_gas_price: txn_data.gas_unit_price().into(),
            txn_max_gas_units: txn_data.max_gas_amount().into(),
            gas_units_remaining: gas_remaining.into(),
            is_simulation,
            is_orderless_txn: txn_data.is_orderless(),
        }
    }

    /// Selects the highest supported variant based on feature flags and BCS-serializes it.
    /// Currently only V1 exists.
    pub fn build(self) -> Vec<u8> {
        let args = EpilogueArgs::V1 {
            fee_statement: self.fee_statement,
            txn_gas_price: self.txn_gas_price,
            txn_max_gas_units: self.txn_max_gas_units,
            gas_units_remaining: self.gas_units_remaining,
            is_simulation: self.is_simulation,
            is_orderless_txn: self.is_orderless_txn,
        };
        bcs::to_bytes(&args).expect("Failed to serialize epilogue arguments")
    }
}

pub(crate) fn run_epilogue(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    serialized_signers: &SerializedSigners,
    gas_remaining: Gas,
    fee_statement: FeeStatement,
    txn_data: &TransactionMetadata,
    traversal_context: &mut TraversalContext,
    is_simulation: bool,
) -> VMResult<()> {
    let builder = EpilogueBuilder::new(fee_statement, txn_data, gas_remaining, is_simulation);
    let serialized_args = vec![
        serialized_signers.sender(),
        serialized_signers
            .fee_payer()
            .unwrap_or(serialized_signers.sender()),
        builder.build(),
    ];

    session
        .execute_function_bypass_visibility(
            &TRANSACTION_VALIDATION_MODULE,
            VERSIONED_EPILOGUE_NAME,
            vec![],
            serialized_args,
            &mut UnmeteredGasMeter,
            traversal_context,
            module_storage,
        )
        .map(|_return_vals| ())
        .map_err(expect_no_verification_errors)?;

    maybe_raise_injected_error(InjectedError::EndOfRunEpilogue)?;

    Ok(())
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    error::{ErrorCode, V2Error},
    extractors::BcsOnly,
    types::V2Response,
};
use aptos_api_types::{AsConverter, Transaction, TransactionOnChainData, UserTransaction};
use aptos_crypto::hash::CryptoHash;
use aptos_types::transaction::SignedTransaction;
use aptos_vm::AptosSimulationVM;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

/// Query parameters for simulation.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct SimulateParams {
    /// If true, the gas unit price in the transaction will be ignored
    /// and the estimated value will be used.
    pub estimate_gas_unit_price: Option<bool>,
    /// If true, the transaction will use a higher price than the
    /// regular estimate (prioritized).
    pub estimate_prioritized_gas_unit_price: Option<bool>,
}

/// POST /v2/transactions/simulate -- simulate a signed transaction (BCS only).
///
/// The transaction must have an *invalid* signature (to prevent accidental execution).
/// Returns the simulated transaction result including gas used, events, and changes.
#[utoipa::path(
    post,
    path = "/v2/transactions/simulate",
    tag = "Transactions",
    params(SimulateParams),
    request_body(content = Vec<u8>, content_type = "application/x-bcs",
        description = "BCS-encoded signed transaction (must have invalid signature)"),
    responses(
        (status = 200, description = "Simulated transaction result", body = Object),
        (status = 400, description = "Invalid input", body = V2Error),
    )
)]
pub async fn simulate_transaction_handler(
    State(ctx): State<V2Context>,
    Query(params): Query<SimulateParams>,
    BcsOnly(versioned): BcsOnly<SignedTransaction>,
) -> Result<Json<V2Response<Vec<UserTransaction>>>, V2Error> {
    let txn = versioned.into_inner();

    // Simulated transactions must NOT have a valid signature
    if txn.verify_signature().is_ok() {
        return Err(V2Error::bad_request(
            ErrorCode::InvalidInput,
            "Simulated transactions must not have a valid signature",
        ));
    }

    // Reject encrypted transactions
    if txn
        .raw_transaction_ref()
        .payload_ref()
        .is_encrypted_variant()
    {
        return Err(V2Error::bad_request(
            ErrorCode::InvalidInput,
            "Encrypted transactions cannot be simulated",
        ));
    }

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let ledger_info = ctx.ledger_info()?;

        // Optionally replace gas unit price with estimate
        let txn = if params.estimate_gas_unit_price.unwrap_or(false)
            || params.estimate_prioritized_gas_unit_price.unwrap_or(false)
        {
            let gas_estimation = ctx
                .inner()
                .estimate_gas_price(&ledger_info)
                .map_err(|e: crate::response::BasicError| V2Error::internal(format!("{}", e)))?;

            let price = if params.estimate_prioritized_gas_unit_price.unwrap_or(false) {
                gas_estimation
                    .prioritized_gas_estimate
                    .unwrap_or(gas_estimation.gas_estimate)
            } else {
                gas_estimation.gas_estimate
            };

            // Rebuild the signed transaction with the new gas price
            let authenticator = txn.authenticator();
            let mut raw = txn.into_raw_transaction();
            raw.set_gas_unit_price(price);
            SignedTransaction::new_signed_transaction(raw, authenticator)
        } else {
            txn
        };

        // Execute simulation
        let state_view = ctx.inner().latest_state_view().map_err(V2Error::internal)?;
        let (vm_status, output) =
            AptosSimulationVM::create_vm_and_simulate_signed_transaction(&txn, &state_view);

        let version = ledger_info.version();
        let exe_status =
            aptos_types::transaction::ExecutionStatus::conmbine_vm_status_for_simulation(
                output.auxiliary_data(),
                output.status().clone(),
            );

        // Build simulated transaction
        let sim_txn = aptos_types::transaction::Transaction::UserTransaction(txn);
        let zero_hash = aptos_crypto::HashValue::zero();
        let info = aptos_types::transaction::TransactionInfo::new(
            sim_txn.hash(),
            zero_hash,
            zero_hash,
            None,
            output.gas_used(),
            exe_status,
            None,
        );

        let events = output.events().to_vec();
        let simulated_txn = TransactionOnChainData {
            version,
            transaction: sim_txn,
            info,
            events,
            accumulator_root_hash: zero_hash,
            changes: output.write_set().clone(),
        };

        // Render to JSON
        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

        let rendered = converter
            .try_into_onchain_transaction(ledger_info.ledger_timestamp.into(), simulated_txn)
            .map_err(V2Error::internal)?;

        // Extract UserTransaction from Transaction
        let user_txn = match rendered {
            Transaction::UserTransaction(mut ut) => {
                // Append VM error message if present
                use aptos_types::vm_status::VMStatus;
                match &vm_status {
                    VMStatus::Error {
                        message: Some(msg), ..
                    }
                    | VMStatus::ExecutionFailure {
                        message: Some(msg), ..
                    } => {
                        ut.info.vm_status +=
                            format!("\nExecution failed with message: {}", msg).as_str();
                    },
                    _ => (),
                }
                ut
            },
            _ => {
                return Err(V2Error::internal(
                    "Simulation produced a non-UserTransaction",
                ));
            },
        };

        Ok(Json(V2Response::new(vec![user_txn], &ledger_info)))
    })
    .await
}

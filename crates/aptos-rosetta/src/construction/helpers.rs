// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    error::{ApiError, ApiResult},
    node_client::{self, NodeClient},
    types::*,
};
use aptos_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    ValidCryptoMaterialStringExt,
};
use aptos_global_constants::adjust_gas_headroom;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_types::{chain_id::ChainId, transaction::SignedTransaction};
use std::convert::TryFrom;

/// Fills in the operator for actions that require it but don't have one on an [InternalOperation]
/// TODO: move this onto [InternalOperation] and not in this file
pub(crate) async fn fill_in_operator(
    rest_client: &dyn NodeClient,
    mut internal_operation: InternalOperation,
) -> ApiResult<InternalOperation> {
    // TODO: Refactor so there's not duplicate code below
    match &mut internal_operation {
        InternalOperation::SetOperator(op) => {
            // If there was no old operator set, and there is only one, we should use that
            if op.old_operator.is_none() {
                let store = node_client::get_account_resource_bcs::<Store>(
                    rest_client,
                    op.owner,
                    "0x1::staking_contract::Store",
                )
                .await?
                .into_inner();
                let staking_contracts = store.staking_contracts;
                if staking_contracts.len() != 1 {
                    let operators: Vec<_> = staking_contracts
                        .iter()
                        .map(|(address, _)| *address)
                        .collect();
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Account has more than one operator, operator must be specified from: {:?}",
                        operators
                    ))));
                } else {
                    // Take the only staking contract
                    op.old_operator = Some(
                        staking_contracts
                            .first()
                            .map(|(address, _)| *address)
                            .unwrap(),
                    );
                }
            }
        },
        InternalOperation::SetVoter(op) => {
            // If there was no operator set, and there is only one, we should use that
            if op.operator.is_none() {
                let store = node_client::get_account_resource_bcs::<Store>(
                    rest_client,
                    op.owner,
                    "0x1::staking_contract::Store",
                )
                .await?
                .into_inner();
                let staking_contracts = store.staking_contracts;
                if staking_contracts.len() != 1 {
                    let operators: Vec<_> = staking_contracts
                        .iter()
                        .map(|(address, _)| address)
                        .collect();
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Account has more than one operator, operator must be specified from: {:?}",
                        operators
                    ))));
                } else {
                    // Take the only staking contract
                    op.operator = Some(
                        staking_contracts
                            .first()
                            .map(|(address, _)| *address)
                            .unwrap(),
                    );
                }
            }
        },
        _ => {},
    }

    Ok(internal_operation)
}

/// Simulates a transaction for gas estimation purposes
///
/// Only the original Ed25519 accounts on Aptos are supported
///
/// Will only simulate if it does not have max gas amount
///
/// Will only estimate gas price
pub(crate) async fn simulate_transaction(
    rest_client: &dyn NodeClient,
    chain_id: ChainId,
    options: &MetadataOptions,
    internal_operation: &InternalOperation,
    sequence_number: u64,
) -> ApiResult<(Amount, u64, u64)> {
    // If we have any missing fields, let's simulate!
    let mut transaction_factory = TransactionFactory::new(chain_id);

    // If we have a gas unit price, let's not estimate
    // TODO: Split into separate function
    if let Some(gas_unit_price) = options.gas_price_per_unit.as_ref() {
        transaction_factory = transaction_factory.with_gas_unit_price(gas_unit_price.0);
    } else {
        let gas_estimation = rest_client.estimate_gas_price().await?.into_inner();

        // Get the priorities, for backwards compatibility, if the API doesn't have the prioritized ones, use the normal one
        let mut gas_price = match options.gas_price_priority.unwrap_or_default() {
            GasPricePriority::Low => gas_estimation
                .deprioritized_gas_estimate
                .unwrap_or(gas_estimation.gas_estimate),
            GasPricePriority::Normal => gas_estimation.gas_estimate,
            GasPricePriority::High => gas_estimation
                .prioritized_gas_estimate
                .unwrap_or(gas_estimation.gas_estimate),
        };

        // We can also provide the multiplier at this point, we mulitply times it, and divide by 100
        if let Some(gas_multiplier) = options.gas_price_multiplier {
            let gas_multiplier = gas_multiplier as u64;
            if let Some(multiplied_price) = gas_price.checked_mul(gas_multiplier) {
                gas_price = multiplied_price.saturating_div(100)
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Gas price multiplier {} causes overflow on the price",
                    gas_multiplier
                ))));
            }
        }

        transaction_factory = transaction_factory.with_gas_unit_price(gas_price);
    }

    // Build up the transaction
    let (txn_payload, sender) = internal_operation.payload()?;
    let unsigned_transaction = transaction_factory
        .payload(txn_payload)
        .sender(sender)
        .sequence_number(sequence_number)
        .build();

    // Read and fill in public key as necessary, this is required for simulation!
    let public_key =
        if let Some(public_key) = options.public_keys.as_ref().and_then(|inner| inner.first()) {
            Ed25519PublicKey::from_encoded_string(&public_key.hex_bytes).map_err(|err| {
                ApiError::InvalidInput(Some(format!(
                    "Public key provided is not parsable {:?}",
                    err
                )))
            })?
        } else {
            return Err(ApiError::InvalidInput(Some(
                "Must provide public_keys for simulation otherwise it can't simulate!".to_string(),
            )));
        };

    // Sign the transaction with a dummy signature of all zeros as required by the API
    let signed_transaction = SignedTransaction::new(
        unsigned_transaction,
        public_key,
        Ed25519Signature::try_from([0u8; 64].as_ref()).expect("Zero signature should always work"),
    );

    // Simulate, filling in the fields that aren't being currently handled
    // This API will always succeed unless 2 conditions
    // 1. The API was going to fail anyways due to a bad transaction e.g. wrong signer, insufficient balance, etc.
    // 2. The used gas price (provided or estimated) * the maximum possible gas is can't be paid e.g. there is no
    //    way for this user to ever pay for this transaction (at that gas price)
    let response = rest_client
        .simulate_bcs_with_gas_estimation(&signed_transaction, true, false)
        .await?;

    let simulated_txn = response.inner();

    // Check that we didn't go over the max gas provided by the API
    if let Some(max_gas_amount) = options.max_gas_amount.as_ref() {
        if max_gas_amount.0 < simulated_txn.info.gas_used() {
            return Err(ApiError::MaxGasFeeTooLow(Some(format!(
                "Max gas amount {} is less than number of actual gas units used {}",
                max_gas_amount.0,
                simulated_txn.info.gas_used()
            ))));
        }
    }

    // Handle any other messages, including out of gas, which means the user has not enough
    // funds to complete the transaction (e.g. the gas price is too high)
    let simulation_status = simulated_txn.info.status();
    if !simulation_status.is_success() {
        // TODO: Fix case for not enough gas to be a better message
        return Err(ApiError::InvalidInput(Some(format!(
            "Transaction failed to simulate with status: {:?}",
            simulation_status
        ))));
    }

    if let Some(user_txn) = simulated_txn.transaction.try_as_signed_user_txn() {
        // This gas price came from the simulation (would be the one from the input if provided)
        let simulated_gas_unit_price = user_txn.gas_unit_price();

        // These two will either be estimated or the original value, so we can just use them exactly
        let max_gas_amount = if let Some(max_gas_amount) = options.max_gas_amount.as_ref() {
            max_gas_amount.0
        } else {
            // If estimating, we want to give headroom to ensure the transaction succeeds
            adjust_gas_headroom(simulated_txn.info.gas_used(), user_txn.max_gas_amount())
        };

        // Multiply the gas price times the max gas amount to use
        let suggested_fee = Amount::suggested_gas_fee(simulated_gas_unit_price, max_gas_amount);

        Ok((suggested_fee, simulated_gas_unit_price, max_gas_amount))
    } else {
        // This should never happen, because the underlying API can't run a non-user transaction
        Err(ApiError::InternalError(Some(format!(
            "Transaction returned by API was not a user transaction: {:?}",
            simulated_txn.transaction
        ))))
    }
}

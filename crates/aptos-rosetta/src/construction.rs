// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Construction APIs
//!
//! The construction APIs break down transactions into composable parts that are
//! used to be generic across blockchains.  A flow of operations can be found
//! in the [specifications](https://www.rosetta-api.org/docs/construction_api_introduction.html)
//!
//! This is broken down in the following flow:
//!
//! * Preprocess (based on operations) gets information to fetch from metadata (on-chain)
//! * Metadata fetches on-chain information e.g. sequence number
//! * Payloads generates an unsigned transaction
//! * Application outside signs the payload from the transaction
//! * Combine puts the signed transaction payload with the unsigned transaction
//! * Submit submits the signed transaction to the blockchain
//!
//! There are also 2 other sometimes used APIs
//! * Derive (get an account from the private key)
//! * Hash (get a hash of the transaction to lookup in mempool)
//!
//! Note: there is an "online" mode and an "offline" mode.  The offline APIs can run without
//! a connection to a full node.  The online ones need a connection to a full node.
//!

use crate::{
    common::{
        check_network, decode_bcs, decode_key, encode_bcs, find_fa_currency, get_account,
        handle_request, native_coin, parse_coin_currency, with_context,
    },
    error::{ApiError, ApiResult},
    types::{InternalOperation, *},
    RosettaContext,
};
use aptos_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    signing_message, ValidCryptoMaterialStringExt,
};
use aptos_global_constants::adjust_gas_headroom;
use aptos_logger::debug;
use aptos_sdk::{move_types::language_storage::TypeTag, transaction_builder::TransactionFactory};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{
        authenticator::AuthenticationKey, RawTransaction, SignedTransaction, TransactionPayload,
    },
};
use serde::de::DeserializeOwned;
use std::{
    convert::TryFrom,
    time::{SystemTime, UNIX_EPOCH},
};
use warp::Filter;

pub fn combine_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "combine")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_combine))
}

pub fn derive_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "derive")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_derive))
}

pub fn hash_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "hash")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_hash))
}

pub fn metadata_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "metadata")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_metadata))
}

pub fn parse_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "parse")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_parse))
}

pub fn payloads_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "payloads")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_payloads))
}

pub fn preprocess_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "preprocess")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_preprocess))
}

pub fn submit_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("construction" / "submit")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_submit))
}

/// Construction combine command (OFFLINE)
///
/// This combines signatures, and a raw transaction
///
/// This currently only supports the original Ed25519 with single signer.
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructioncombine)
async fn construction_combine(
    request: ConstructionCombineRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionCombineResponse> {
    debug!("/construction/combine {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // Decode the unsigned transaction from BCS in the input
    let unsigned_txn: RawTransaction =
        decode_bcs(&request.unsigned_transaction, "UnsignedTransaction")?;

    // Single signer only supported for now
    // TODO: Support multi-agent / multi-signer?
    if request.signatures.len() != 1 {
        return Err(ApiError::UnsupportedSignatureCount(Some(
            request.signatures.len(),
        )));
    }

    let signature = &request.signatures[0];

    // Only support Ed25519
    if signature.signature_type != SignatureType::Ed25519
        || signature.public_key.curve_type != CurveType::Edwards25519
    {
        return Err(ApiError::InvalidSignatureType);
    }

    // Decode the key and signature accordingly
    let public_key: Ed25519PublicKey =
        decode_key(&signature.public_key.hex_bytes, "Ed25519PublicKey")?;
    let signature: Ed25519Signature = decode_key(&signature.hex_bytes, "Ed25519Signature")?;

    // Combine them into a signed transaction, and encode it as BCS to return
    let signed_txn = SignedTransaction::new(unsigned_txn, public_key, signature);

    Ok(ConstructionCombineResponse {
        signed_transaction: encode_bcs(&signed_txn)?,
    })
}

/// Construction derive command (OFFLINE)
///
/// Derive account address from Public key
/// Note: This only works for new accounts.  After the account is created, all APIs should provide
/// both account and key.
///
/// Note: if the accounts are handled ONLY by Rosetta, then this will always work.  It only stops working
/// if it is one of many other types of keys / a rotated account.
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionderive)
async fn construction_derive(
    request: ConstructionDeriveRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionDeriveResponse> {
    debug!("/construction/derive {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // The input must be an Ed25519 Public key and will only derive the Address for the original
    // Aptos Ed25519 authentication scheme
    let public_key: Ed25519PublicKey =
        decode_key(&request.public_key.hex_bytes, "Ed25519PublicKey")?;
    let address = AuthenticationKey::ed25519(&public_key).account_address();

    Ok(ConstructionDeriveResponse {
        account_identifier: AccountIdentifier::base_account(address),
    })
}

/// Construction hash command (OFFLINE)
///
/// Hash a transaction to get it's identifier for lookup in mempool
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionhash)
async fn construction_hash(
    request: ConstructionHashRequest,
    server_context: RosettaContext,
) -> ApiResult<TransactionIdentifierResponse> {
    debug!("/construction/hash {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // Decode the SignedTransaction and hash it accordingly.  This in theory works for any transaction
    // but it is expected to only be UserTransactions
    let signed_transaction: SignedTransaction =
        decode_bcs(&request.signed_transaction, "SignedTransaction")?;

    Ok(TransactionIdentifierResponse {
        transaction_identifier: signed_transaction.committed_hash().into(),
    })
}

/// Fills in the operator for actions that require it but don't have one on an [InternalOperation]
/// TODO: move this onto [InternalOperation] and not in this file
async fn fill_in_operator(
    rest_client: &aptos_rest_client::Client,
    mut internal_operation: InternalOperation,
) -> ApiResult<InternalOperation> {
    // TODO: Refactor so there's not duplicate code below
    match &mut internal_operation {
        InternalOperation::SetOperator(op) => {
            // If there was no old operator set, and there is only one, we should use that
            if op.old_operator.is_none() {
                let store = rest_client
                    .get_account_resource_bcs::<Store>(op.owner, "0x1::staking_contract::Store")
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
                let store = rest_client
                    .get_account_resource_bcs::<Store>(op.owner, "0x1::staking_contract::Store")
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
async fn simulate_transaction(
    rest_client: &aptos_rest_client::Client,
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

/// Construction metadata command
///
/// Retrieves sequence number, gas price, max gas, gas estimate for the transaction
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionmetadata)
async fn construction_metadata(
    request: ConstructionMetadataRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionMetadataResponse> {
    debug!("/construction/metadata {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    let rest_client = server_context.rest_client()?;
    let address = request.options.internal_operation.sender();
    let response = get_account(&rest_client, address).await?;

    // Ensure this network really is the one we expect it to be
    if server_context.chain_id.id() != response.state().chain_id {
        return Err(ApiError::ChainIdMismatch);
    }

    // Retrieve the sequence number from the rest server if one wasn't provided
    let sequence_number = if let Some(sequence_number) = request.options.sequence_number {
        sequence_number.0
    } else {
        response.inner().sequence_number
    };

    // We have to cheat the set operator and set voter operations right here
    let internal_operation = fill_in_operator(
        rest_client.as_ref(),
        request.options.internal_operation.clone(),
    )
    .await?;

    // If both are present, we skip simulation
    let (suggested_fee, gas_unit_price, max_gas_amount) = simulate_transaction(
        rest_client.as_ref(),
        server_context.chain_id,
        &request.options,
        &internal_operation,
        sequence_number,
    )
    .await?;

    Ok(ConstructionMetadataResponse {
        metadata: ConstructionMetadata {
            sequence_number: sequence_number.into(),
            max_gas_amount: max_gas_amount.into(),
            gas_price_per_unit: gas_unit_price.into(),
            expiry_time_secs: request.options.expiry_time_secs,
            internal_operation,
        },
        suggested_fee: vec![suggested_fee],
    })
}

/// Construction parse command (OFFLINE)
///
/// Parses operations from a transaction, used for verifying transaction construction
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionparse)
async fn construction_parse(
    request: ConstructionParseRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionParseResponse> {
    debug!("/construction/parse {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // For signed transactions, we can pull the signers and the raw transaction
    let metadata;
    let (account_identifier_signers, unsigned_txn) = if request.signed {
        let signed_txn: SignedTransaction = decode_bcs(&request.transaction, "SignedTransaction")?;
        metadata = Some(ConstructionParseMetadata {
            unsigned_transaction: None,
            signed_transaction: Some(signed_txn.clone()),
        });
        let mut account_identifier_signers: Vec<_> =
            vec![AccountIdentifier::base_account(signed_txn.sender())];

        for account in signed_txn.authenticator().secondary_signer_addresses() {
            account_identifier_signers.push(AccountIdentifier::base_account(account))
        }

        (
            Some(account_identifier_signers),
            signed_txn.into_raw_transaction(),
        )
    } else {
        // For unsigned transactions,w e can only pull the transaction
        let unsigned_txn: RawTransaction = decode_bcs(&request.transaction, "UnsignedTransaction")?;
        metadata = Some(ConstructionParseMetadata {
            unsigned_transaction: Some(unsigned_txn.clone()),
            signed_transaction: None,
        });
        (None, unsigned_txn)
    };

    // The sender however should always be present, even if not signed
    let sender = unsigned_txn.sender();

    // This is messy, but all we can do is to manually go through and check the entry functions associated to convert to Rosetta operations
    // TODO: We should centralize all this operation -> entry function / entry function -> operation code
    let operations = match unsigned_txn.into_payload() {
        TransactionPayload::EntryFunction(inner) => {
            let (module, function_name, type_args, args) = inner.into_inner();

            match (
                *module.address(),
                module.name().as_str(),
                function_name.as_str(),
            ) {
                (AccountAddress::ONE, COIN_MODULE, TRANSFER_FUNCTION)
                | (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_COINS_FUNCTION) => {
                    parse_transfer_operation(&server_context, sender, &type_args, &args)?
                },
                (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_FUNCTION) => {
                    parse_account_transfer_operation(sender, &type_args, &args)?
                },
                (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, CREATE_ACCOUNT_FUNCTION) => {
                    parse_create_account_operation(sender, &type_args, &args)?
                },
                (AccountAddress::ONE, PRIMARY_FUNGIBLE_STORE_MODULE, TRANSFER_FUNCTION)
                | (AccountAddress::ONE, APTOS_ACCOUNT_MODULE, TRANSFER_FUNGIBLE_ASSETS_FUNCTION) => {
                    parse_primary_fa_transfer_operation(&server_context, sender, &type_args, &args)?
                },
                (AccountAddress::ONE, FUNGIBLE_ASSET_MODULE, TRANSFER_FUNCTION) => {
                    parse_fa_transfer_operation(&server_context, sender, &type_args, &args)?
                },
                (
                    AccountAddress::ONE,
                    STAKING_CONTRACT_MODULE,
                    SWITCH_OPERATOR_WITH_SAME_COMMISSION_FUNCTION,
                ) => parse_set_operator_operation(sender, &type_args, &args)?,
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, UPDATE_VOTER_FUNCTION) => {
                    parse_set_voter_operation(sender, &type_args, &args)?
                },
                (
                    AccountAddress::ONE,
                    STAKING_CONTRACT_MODULE,
                    CREATE_STAKING_CONTRACT_FUNCTION,
                ) => parse_create_stake_pool_operation(sender, &type_args, &args)?,
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, RESET_LOCKUP_FUNCTION) => {
                    parse_reset_lockup_operation(sender, &type_args, &args)?
                },
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, UPDATE_COMMISSION_FUNCTION) => {
                    parse_update_commission_operation(sender, &type_args, &args)?
                },
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, UNLOCK_STAKE_FUNCTION) => {
                    parse_unlock_stake_operation(sender, &type_args, &args)?
                },
                (
                    AccountAddress::ONE,
                    STAKING_CONTRACT_MODULE,
                    DISTRIBUTE_STAKING_REWARDS_FUNCTION,
                ) => parse_distribute_staking_rewards_operation(sender, &type_args, &args)?,
                (
                    AccountAddress::ONE,
                    DELEGATION_POOL_MODULE,
                    DELEGATION_POOL_ADD_STAKE_FUNCTION,
                ) => parse_delegation_pool_add_stake_operation(sender, &type_args, &args)?,
                (
                    AccountAddress::ONE,
                    DELEGATION_POOL_MODULE,
                    DELEGATION_POOL_WITHDRAW_FUNCTION,
                ) => parse_delegation_pool_withdraw_operation(sender, &type_args, &args)?,
                (AccountAddress::ONE, DELEGATION_POOL_MODULE, DELEGATION_POOL_UNLOCK_FUNCTION) => {
                    parse_delegation_pool_unlock_operation(sender, &type_args, &args)?
                },
                _ => {
                    return Err(ApiError::TransactionParseError(Some(format!(
                        "Unsupported entry function type {:x}::{}::{}",
                        module.address(),
                        module.name(),
                        function_name
                    ))));
                },
            }
        },
        payload => {
            return Err(ApiError::TransactionParseError(Some(format!(
                "Unsupported transaction payload type {:?}",
                payload
            ))))
        },
    };

    Ok(ConstructionParseResponse {
        operations,
        account_identifier_signers,
        metadata,
    })
}

/// Parses 0x1::aptos_account::create(auth_key: address)
fn parse_create_account_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    // There are no typeargs for create account
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Create account should not have type arguments: {:?}",
            type_args
        ))));
    }

    // Create account
    if let Some(encoded_address) = args.first() {
        let new_address: AccountAddress = bcs::from_bytes(encoded_address)?;

        Ok(vec![Operation::create_account(
            0,
            None,
            new_address,
            sender,
        )])
    } else {
        Err(ApiError::InvalidOperations(Some(
            "Create account doesn't have an address argument".to_string(),
        )))
    }
}

/// Parses 0x1::coin::transfer<CoinType>(receiver: address, amount: u64)
fn parse_transfer_operation(
    server_context: &RosettaContext,
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    let mut operations = Vec::new();

    // Check coin is the native coin
    let currency = match type_args.first() {
        Some(TypeTag::Struct(struct_tag)) => parse_coin_currency(server_context, struct_tag)?,
        _ => {
            return Err(ApiError::TransactionParseError(Some(
                "No coin type in transfer".to_string(),
            )))
        },
    };

    // Retrieve the args for the operations
    let receiver: AccountAddress = if let Some(receiver) = args.first() {
        bcs::from_bytes(receiver)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No receiver in transfer".to_string(),
        )));
    };
    let amount: u64 = if let Some(amount) = args.get(1) {
        bcs::from_bytes(amount)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No amount in transfer".to_string(),
        )));
    };

    operations.push(Operation::withdraw(
        0,
        None,
        AccountIdentifier::base_account(sender),
        currency.clone(),
        amount,
    ));
    operations.push(Operation::deposit(
        1,
        None,
        AccountIdentifier::base_account(receiver),
        currency,
        amount,
    ));
    Ok(operations)
}

/// Parses 0x1::aptos_account::transfer(receiver: address, amount: u64)
fn parse_account_transfer_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    // There are no typeargs for account transfer
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Account transfer should not have type arguments: {:?}",
            type_args
        ))));
    }
    let mut operations = Vec::new();

    // Retrieve the args for the operations
    // TODO: This is the same as coin::transfer, we should combine them
    let receiver: AccountAddress = if let Some(receiver) = args.first() {
        bcs::from_bytes(receiver)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No receiver in account transfer".to_string(),
        )));
    };
    let amount: u64 = if let Some(amount) = args.get(1) {
        bcs::from_bytes(amount)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No amount in account transfer".to_string(),
        )));
    };

    operations.push(Operation::withdraw(
        0,
        None,
        AccountIdentifier::base_account(sender),
        native_coin(),
        amount,
    ));
    operations.push(Operation::deposit(
        1,
        None,
        AccountIdentifier::base_account(receiver),
        native_coin(),
        amount,
    ));
    Ok(operations)
}

/// Parses 0x1::primary_fungible_store::transfer(metadata: address, receiver: address, amount: u64)
/// or 0x1::aptos_account::transfer_fungible_assets(metadata: address, receiver: address, amount: u64)
fn parse_primary_fa_transfer_operation(
    server_context: &RosettaContext,
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    // There should be one type arg
    if type_args.len() != 1 {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Primary fungible store transfer should have one type argument: {:?}",
            type_args
        ))));
    }
    let mut operations = Vec::new();

    // Retrieve the args for the operations
    let metadata: AccountAddress = if let Some(metadata) = args.first() {
        bcs::from_bytes(metadata)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No metadata address in primary fungible transfer".to_string(),
        )));
    };
    let receiver: AccountAddress = if let Some(receiver) = args.get(1) {
        bcs::from_bytes(receiver)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No receiver address in primary fungible transfer".to_string(),
        )));
    };
    let amount: u64 = if let Some(amount) = args.get(2) {
        bcs::from_bytes(amount)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No amount in primary fungible transfer".to_string(),
        )));
    };

    // Grab currency accordingly

    let maybe_currency = find_fa_currency(&server_context.currencies, metadata);

    if let Some(currency) = maybe_currency {
        operations.push(Operation::withdraw(
            0,
            None,
            AccountIdentifier::base_account(sender),
            currency.clone(),
            amount,
        ));
        operations.push(Operation::deposit(
            1,
            None,
            AccountIdentifier::base_account(receiver),
            currency.clone(),
            amount,
        ));
        Ok(operations)
    } else {
        Err(ApiError::UnsupportedCurrency(Some(metadata.to_string())))
    }
}

/// Parses 0x1::fungible_asset::transfer(metadata: address, receiver: address, amount: u64)
///
/// This is only for using directly from a store, please prefer using primary fa.
fn parse_fa_transfer_operation(
    server_context: &RosettaContext,
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    // There is one type arg for the object
    if type_args.len() != 1 {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Fungible asset transfer should have one type argument: {:?}",
            type_args
        ))));
    }
    let mut operations = Vec::new();

    // Retrieve the args for the operations
    let metadata: AccountAddress = if let Some(metadata) = args.first() {
        bcs::from_bytes(metadata)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No metadata address in fungible asset transfer".to_string(),
        )));
    };
    let receiver: AccountAddress = if let Some(receiver) = args.get(1) {
        bcs::from_bytes(receiver)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No receiver address in fungible asset transfer".to_string(),
        )));
    };
    let amount: u64 = if let Some(amount) = args.get(2) {
        bcs::from_bytes(amount)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No amount in fungible transfer".to_string(),
        )));
    };

    // Grab currency accordingly

    let maybe_currency = find_fa_currency(&server_context.currencies, metadata);

    if let Some(currency) = maybe_currency {
        operations.push(Operation::withdraw(
            0,
            None,
            AccountIdentifier::base_account(sender),
            currency.clone(),
            amount,
        ));
        operations.push(Operation::deposit(
            1,
            None,
            AccountIdentifier::base_account(receiver),
            currency.clone(),
            amount,
        ));
        Ok(operations)
    } else {
        Err(ApiError::UnsupportedCurrency(Some(metadata.to_string())))
    }
}

/// Parses a specific BCS function argument to the given type
pub fn parse_function_arg<T: DeserializeOwned>(
    name: &str,
    args: &[Vec<u8>],
    index: usize,
) -> ApiResult<T> {
    if let Some(arg) = args.get(index) {
        if let Ok(arg) = bcs::from_bytes::<T>(arg) {
            return Ok(arg);
        }
    }

    Err(ApiError::InvalidInput(Some(format!(
        "Argument {} of {} failed to parse",
        index, name
    ))))
}

/// Parses 0x1::staking_contract::switch_operator_with_same_commission(old_operator: address, new_operator: address)
pub fn parse_set_operator_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Set operator should not have type arguments: {:?}",
            type_args
        ))));
    }

    let old_operator = parse_function_arg("set_operator", args, 0)?;
    let new_operator = parse_function_arg("set_operator", args, 1)?;
    Ok(vec![Operation::set_operator(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(old_operator)),
        AccountIdentifier::base_account(new_operator),
        None,
    )])
}

/// Parses 0x1::staking_contract::update_voter(operator: address, new_voter: address)
pub fn parse_set_voter_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Set voter should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator = parse_function_arg("set_voter", args, 0)?;
    let new_voter = parse_function_arg("set_voter", args, 1)?;
    Ok(vec![Operation::set_voter(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(operator)),
        AccountIdentifier::base_account(new_voter),
    )])
}

/// Parses 0x1::staking_contract::create_staking_contract(operator: address, voter: address, amount: u64, commission_percentage: u64)
pub fn parse_create_stake_pool_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Create stake pool should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator = parse_function_arg("create_stake_pool", args, 0)?;
    let voter = parse_function_arg("create_stake_pool", args, 1)?;
    let amount: u64 = parse_function_arg("create_stake_pool", args, 2)?;
    let commission_percentage: u64 = parse_function_arg("create_stake_pool", args, 3)?;
    Ok(vec![Operation::create_stake_pool(
        0,
        None,
        sender,
        Some(operator),
        Some(voter),
        Some(amount),
        Some(commission_percentage),
    )])
}

/// Parses 0x1::staking_contract::reset_lockup(operator: address)
pub fn parse_reset_lockup_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Reset lockup should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator: AccountAddress = parse_function_arg("reset_lockup", args, 0)?;
    Ok(vec![Operation::reset_lockup(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(operator)),
    )])
}

/// Parses 0x1::staking_contract::unlock_stake(operator: address, amount: u64)
pub fn parse_unlock_stake_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Unlock stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator: AccountAddress = parse_function_arg("unlock_stake", args, 0)?;
    let amount: u64 = parse_function_arg("unlock_stake", args, 1)?;

    Ok(vec![Operation::unlock_stake(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(operator)),
        Some(amount),
    )])
}

/// Parses 0x1::staking_contract::update_commission(operator: address, new_commission_percentage: u64)
pub fn parse_update_commission_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Unlock stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let operator: AccountAddress = parse_function_arg("update_commision", args, 0)?;
    let new_commission_percentage: u64 = parse_function_arg("update_commision", args, 1)?;

    Ok(vec![Operation::update_commission(
        0,
        None,
        sender,
        Some(AccountIdentifier::base_account(operator)),
        Some(new_commission_percentage),
    )])
}

/// Parses 0x1::staking_contract::distribute(staker: address, operator: address)
pub fn parse_distribute_staking_rewards_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Distribute should not have type arguments: {:?}",
            type_args
        ))));
    }

    let staker: AccountAddress = parse_function_arg("distribute_staking_rewards", args, 0)?;
    let operator: AccountAddress = parse_function_arg("distribute_staking_rewards", args, 1)?;

    Ok(vec![Operation::distribute_staking_rewards(
        0,
        None,
        sender,
        AccountIdentifier::base_account(operator),
        AccountIdentifier::base_account(staker),
    )])
}

/// Parses 0x1::delegation_pool::add_stake(pool_address: address, amount: u64)
pub fn parse_delegation_pool_add_stake_operation(
    delegator: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "add_delegated_stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let pool_address: AccountAddress = parse_function_arg("add_delegated_stake", args, 0)?;
    let amount: u64 = parse_function_arg("add_delegated_stake", args, 1)?;

    Ok(vec![Operation::add_delegated_stake(
        0,
        None,
        delegator,
        AccountIdentifier::base_account(pool_address),
        Some(amount),
    )])
}

/// Parses 0x1::delegation_pool::unlock(pool_address: address, amount: u64)
pub fn parse_delegation_pool_unlock_operation(
    delegator: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Unlock delegated stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let pool_address: AccountAddress = parse_function_arg("unlock_delegated_stake", args, 0)?;
    let amount: u64 = parse_function_arg("unlock_delegated_stake", args, 1)?;

    Ok(vec![Operation::unlock_delegated_stake(
        0,
        None,
        delegator,
        AccountIdentifier::base_account(pool_address),
        Some(amount),
    )])
}

/// Parses 0x1::delegation_pool::withdraw(pool_address: address, amount: u64)
pub fn parse_delegation_pool_withdraw_operation(
    delegator: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "add_delegated_stake should not have type arguments: {:?}",
            type_args
        ))));
    }

    let pool_address: AccountAddress = parse_function_arg("withdraw_undelegated", args, 0)?;
    let amount: u64 = parse_function_arg("withdraw_undelegated", args, 1)?;

    Ok(vec![Operation::withdraw_undelegated_stake(
        0,
        None,
        delegator,
        AccountIdentifier::base_account(pool_address),
        Some(amount),
    )])
}

/// Construction payloads command (OFFLINE)
///
/// Constructs payloads for given known operations.  This converts Rosetta [Operation]s to a [RawTransaction]
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionpayloads)
async fn construction_payloads(
    request: ConstructionPayloadsRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionPayloadsResponse> {
    debug!("/construction/payloads {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // Retrieve the real operation we're doing, this identifies the sub-operations to a function
    let mut operation = InternalOperation::extract(&server_context, &request.operations)?;

    // For some reason, metadata is optional on the Rosetta spec, we enforce it here, otherwise we
    // can't build the [RawTransaction] offline.
    let metadata = if let Some(ref metadata) = request.metadata {
        metadata
    } else {
        return Err(ApiError::MissingPayloadMetadata);
    };

    // This is a hack to ensure that the payloads actually have overridden operators if not provided
    // It ensures that the operations provided match the metadata provided.
    // TODO: Move this to a separate function
    match &mut operation {
        InternalOperation::CreateAccount(_) => {
            if operation != metadata.internal_operation {
                return Err(ApiError::InvalidInput(Some(format!(
                    "CreateAccount operation doesn't match metadata {:?} vs {:?}",
                    operation, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::Transfer(_) => {
            if operation != metadata.internal_operation {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Transfer operation doesn't match metadata {:?} vs {:?}",
                    operation, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::SetOperator(inner) => {
            if let InternalOperation::SetOperator(ref metadata_op) = metadata.internal_operation {
                if inner.owner == metadata_op.owner
                    && inner.new_operator == metadata_op.new_operator
                {
                    if inner.old_operator.is_none() {
                        inner.old_operator = metadata_op.old_operator;
                    }
                } else {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Set operator operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Set operator operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::SetVoter(inner) => {
            if let InternalOperation::SetVoter(ref metadata_op) = metadata.internal_operation {
                if inner.owner == metadata_op.owner && inner.new_voter == metadata_op.new_voter {
                    if inner.operator.is_none() {
                        inner.operator = metadata_op.operator;
                    }
                } else {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Set voter operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Set voter operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::InitializeStakePool(_) => {
            if operation != metadata.internal_operation {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Initialize stake pool doesn't match metadata {:?} vs {:?}",
                    operation, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::ResetLockup(inner) => {
            if let InternalOperation::ResetLockup(ref metadata_op) = metadata.internal_operation {
                if inner.owner != metadata_op.owner || inner.operator != metadata_op.operator {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Reset lockup operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Reset lockup operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::UnlockStake(inner) => {
            if let InternalOperation::UnlockStake(ref metadata_op) = metadata.internal_operation {
                if inner.owner != metadata_op.owner || inner.operator != metadata_op.operator {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Unlock stake operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Unlock stake operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::UpdateCommission(inner) => {
            if let InternalOperation::UpdateCommission(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.owner != metadata_op.owner || inner.operator != metadata_op.operator {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Update commission operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Update commission operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::DistributeStakingRewards(inner) => {
            if let InternalOperation::DistributeStakingRewards(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.operator != metadata_op.operator || inner.staker != metadata_op.staker {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Distribute staking rewards operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Distribute staking rewards operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::AddDelegatedStake(inner) => {
            if let InternalOperation::AddDelegatedStake(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.delegator != metadata_op.delegator
                    || inner.pool_address != metadata_op.pool_address
                {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "AddDelegatedStake internal operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "InternalOperation::AddDelegatedStake doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::UnlockDelegatedStake(inner) => {
            if let InternalOperation::UnlockDelegatedStake(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.delegator != metadata_op.delegator
                    || inner.pool_address != metadata_op.pool_address
                {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Unlock delegated stake operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Unlock delegated stake operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
        InternalOperation::WithdrawUndelegated(inner) => {
            if let InternalOperation::WithdrawUndelegated(ref metadata_op) =
                metadata.internal_operation
            {
                if inner.delegator != metadata_op.delegator
                    || inner.pool_address != metadata_op.pool_address
                {
                    return Err(ApiError::InvalidInput(Some(format!(
                        "Withdraw undelegated operation doesn't match metadata {:?} vs {:?}",
                        inner, metadata.internal_operation
                    ))));
                }
            } else {
                return Err(ApiError::InvalidInput(Some(format!(
                    "Withdraw undelegated operation doesn't match metadata {:?} vs {:?}",
                    inner, metadata.internal_operation
                ))));
            }
        },
    }

    // Encode operation
    let (txn_payload, sender) = operation.payload()?;

    // Build the transaction and make it ready for signing
    let transaction_factory = TransactionFactory::new(server_context.chain_id)
        .with_gas_unit_price(metadata.gas_price_per_unit.0)
        .with_max_gas_amount(metadata.max_gas_amount.0);

    let mut txn_builder = transaction_factory
        .payload(txn_payload)
        .sender(sender)
        .sequence_number(metadata.sequence_number.0);

    // Default expiry is 30 seconds from right now
    if let Some(expiry_time_secs) = metadata.expiry_time_secs {
        txn_builder = txn_builder.expiration_timestamp_secs(expiry_time_secs.0)
    }
    let unsigned_transaction = txn_builder.build();

    // Build a signing message so that an external signer can sign with Ed25519 without knowing BCS
    let signing_message = hex::encode(signing_message(&unsigned_transaction).map_err(|err| {
        ApiError::InvalidInput(Some(format!(
            "Invalid transaction, can't build into a signing message {}",
            err
        )))
    })?);
    let payload = SigningPayload {
        account_identifier: AccountIdentifier::base_account(sender),
        hex_bytes: signing_message,
        signature_type: Some(SignatureType::Ed25519),
    };

    // Transaction is both the unsigned transaction and the payload
    Ok(ConstructionPayloadsResponse {
        unsigned_transaction: encode_bcs(&unsigned_transaction)?,
        payloads: vec![payload],
    })
}

/// Construction preprocess command (OFFLINE)
///
/// This creates the request needed to fetch metadata.  It basically verifies that the inputs are
/// valid for calling on-chain data.
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionpreprocess)
async fn construction_preprocess(
    request: ConstructionPreprocessRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionPreprocessResponse> {
    debug!("/construction/preprocess {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // Determine the actual operation from the collection of Rosetta [Operation]
    let internal_operation = InternalOperation::extract(&server_context, &request.operations)?;

    // Provide the accounts that need public keys (there's only one supported today)
    let required_public_keys = vec![AccountIdentifier::base_account(internal_operation.sender())];

    // Verify that the max gas value is valid
    if let Some(max_gas) = request
        .metadata
        .as_ref()
        .and_then(|inner| inner.max_gas_amount)
    {
        if max_gas.0 < 1 {
            return Err(ApiError::InvalidInput(Some(
                "Cannot have a max gas amount less than 1".to_string(),
            )));
        }
    }

    // Verify that expiration time is valid
    if let Some(expiry_time_secs) = request
        .metadata
        .as_ref()
        .and_then(|inner| inner.expiry_time_secs)
    {
        // Probably should be greater than now + some amount of time, but for now it's valid
        if expiry_time_secs.0
            <= SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|err| {
                    ApiError::InternalError(Some(format!("Failed to get current time {}", err)))
                })?
                .as_secs()
        {
            return Err(ApiError::InvalidInput(Some(
                "Expiry time secs is in the past, please provide a Unix timestamp in the future"
                    .to_string(),
            )));
        }
    }

    // Check gas input options
    let public_keys = request
        .metadata
        .as_ref()
        .and_then(|inner| inner.public_keys.as_ref());

    // A public key can be provided for simulation, otherwise, a max gas amount would be given.
    if request
        .metadata
        .as_ref()
        .and_then(|inner| inner.max_gas_amount)
        .is_none()
        && public_keys
            .as_ref()
            .map(|inner| inner.is_empty())
            .unwrap_or(false)
    {
        return Err(ApiError::InvalidInput(Some(
            "Must provide either max gas amount or public keys to estimate max gas amount"
                .to_string(),
        )));
    }

    // Convert it to an input to the metadata call
    // TODO: Refactor so that it only does `request.metadata.as_ref()` once
    Ok(ConstructionPreprocessResponse {
        options: MetadataOptions {
            internal_operation,
            max_gas_amount: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.max_gas_amount),
            gas_price_per_unit: request.metadata.as_ref().and_then(|inner| inner.gas_price),
            expiry_time_secs: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.expiry_time_secs),
            sequence_number: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.sequence_number),
            public_keys: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.public_keys.clone()),
            gas_price_multiplier: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.gas_price_multiplier),
            gas_price_priority: request
                .metadata
                .as_ref()
                .and_then(|inner| inner.gas_price_priority),
        },
        required_public_keys,
    })
}

/// Construction submit command (OFFLINE)
///
/// Submits a transaction to the blockchain
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionsubmit)
async fn construction_submit(
    request: ConstructionSubmitRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionSubmitResponse> {
    debug!("/construction/submit {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    let rest_client = server_context.rest_client()?;

    // Submits the transaction, and returns the hash of the transaction
    let txn: SignedTransaction = decode_bcs(&request.signed_transaction, "SignedTransaction")?;
    let hash = txn.committed_hash();
    rest_client.submit_bcs(&txn).await?;
    Ok(ConstructionSubmitResponse {
        transaction_identifier: hash.into(),
    })
}

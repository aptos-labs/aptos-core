// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Construction APIs
//!
//! The construction APIs break down transactions into composable parts that are
//! used to be generic across blockchains.  A flow of operations can be found
//! in the [specifications](https://www.rosetta-api.org/docs/construction_api_introduction.html)
//!
//! This is broken down in the following flow:
//!
//! * Preprocess (based on operations) gets information to fetch from metadata (onchchain)
//! * Metadata fetches onchain information e.g. sequence number
//! * Payloads generates an unsigned transaction
//! * Application outside signs the payload from the transactino
//! * Combine puts the signed transaction payload with the unsigned transaction
//! * Submit submits the signed transaciton to the blockchain
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
        check_network, decode_bcs, decode_key, encode_bcs, get_account, handle_request,
        native_coin, with_context,
    },
    error::{ApiError, ApiResult},
    types::{InternalOperation, *},
    RosettaContext,
};
use aptos_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    signing_message, ValidCryptoMaterialStringExt,
};
use aptos_logger::debug;
use aptos_sdk::{
    move_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    },
    transaction_builder::TransactionFactory,
};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{
        authenticator::AuthenticationKey, RawTransaction, SignedTransaction, TransactionPayload,
    },
};
use std::convert::TryFrom;
use warp::Filter;

pub fn combine_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("construction" / "combine")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_combine))
}

pub fn derive_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("construction" / "derive")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_derive))
}

pub fn hash_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("construction" / "hash")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_hash))
}

pub fn metadata_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("construction" / "metadata")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_metadata))
}

pub fn parse_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("construction" / "parse")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_parse))
}

pub fn payloads_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("construction" / "payloads")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_payloads))
}

pub fn preprocess_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("construction" / "preprocess")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_preprocess))
}

pub fn submit_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("construction" / "submit")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(construction_submit))
}

/// Construction combine command (OFFLINE)
///
/// This combines signatures, and a raw txn
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructioncombine)
async fn construction_combine(
    request: ConstructionCombineRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionCombineResponse> {
    debug!("/construction/combine {:?}", request);
    check_network(request.network_identifier, &server_context)?;

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

    if signature.signature_type != SignatureType::Ed25519
        || signature.public_key.curve_type != CurveType::Edwards25519
    {
        return Err(ApiError::InvalidSignatureType);
    }

    let public_key: Ed25519PublicKey =
        decode_key(&signature.public_key.hex_bytes, "Ed25519PublicKey")?;
    let signature: Ed25519Signature = decode_key(&signature.hex_bytes, "Ed25519Signature")?;

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
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionderive)
async fn construction_derive(
    request: ConstructionDeriveRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionDeriveResponse> {
    debug!("/construction/derive {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    let public_key: Ed25519PublicKey =
        decode_key(&request.public_key.hex_bytes, "Ed25519PublicKey")?;
    let address = AuthenticationKey::ed25519(&public_key).derived_address();

    Ok(ConstructionDeriveResponse {
        account_identifier: address.into(),
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

    let signed_transaction: SignedTransaction =
        decode_bcs(&request.signed_transaction, "SignedTransaction")?;

    Ok(TransactionIdentifierResponse {
        transaction_identifier: signed_transaction.committed_hash().into(),
    })
}

const MAX_GAS_UNITS_PER_REQUEST: u64 = 1_000_000;

/// Construction metadata command
///
/// Retrieve sequence number for submitting transactions
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

    let sequence_number = if let Some(sequence_number) = request.options.sequence_number {
        sequence_number.0
    } else {
        // Retrieve the sequence number from the rest server if one wasn't provided
        response.inner().sequence_number
    };

    // Determine the gas price (either provided from upstream, or through estimation)
    let gas_price_per_unit = if let Some(gas_price) = request.options.gas_price_per_unit {
        gas_price.0
    } else {
        rest_client
            .estimate_gas_price()
            .await?
            .into_inner()
            .gas_estimate
    };

    // Determine max gas by simulation if it isn't provided
    let max_gas_amount = if let Some(max_gas) = request.options.max_gas_amount {
        max_gas.0
    } else {
        let account_balance = rest_client
            .get_account_balance(address)
            .await
            .map_err(|err| ApiError::GasEstimationFailed(Some(err.to_string())))?
            .into_inner();

        let maximum_possible_gas = std::cmp::min(
            account_balance.coin.value.0 / gas_price_per_unit,
            MAX_GAS_UNITS_PER_REQUEST,
        );
        let transaction_factory = TransactionFactory::new(server_context.chain_id)
            .with_gas_unit_price(gas_price_per_unit)
            .with_max_gas_amount(maximum_possible_gas);

        let (txn_payload, sender) = request.options.internal_operation.payload()?;
        let unsigned_transaction = transaction_factory
            .payload(txn_payload)
            .sender(sender)
            .sequence_number(sequence_number)
            .build();

        let public_key = if let Some(public_key) = request
            .options
            .public_keys
            .as_ref()
            .and_then(|inner| inner.first())
        {
            Ed25519PublicKey::from_encoded_string(&public_key.hex_bytes).map_err(|err| {
                ApiError::InvalidInput(Some(format!(
                    "Public key provided is not parsable {:?}",
                    err
                )))
            })?
        } else {
            return Err(ApiError::InvalidInput(Some(
                "Must provide public_keys with max_gas_amount".to_string(),
            )));
        };
        let signed_transaction = SignedTransaction::new(
            unsigned_transaction,
            public_key,
            Ed25519Signature::try_from([0u8; 64].as_ref()).unwrap(),
        );

        let request = rest_client
            .simulate_bcs(&signed_transaction)
            .await?
            .into_inner();

        if request.info.status().is_success() {
            request.info.gas_used()
        } else {
            return Err(ApiError::VmError(Some(format!(
                "Transaction simulation for gas failed with {:?}",
                request.info.status()
            ))));
        }
    };

    let suggested_fee = Amount {
        value: format!("-{}", gas_price_per_unit.saturating_mul(max_gas_amount)),
        currency: native_coin(),
    };

    Ok(ConstructionMetadataResponse {
        metadata: ConstructionMetadata {
            sequence_number: sequence_number.into(),
            max_gas_amount: max_gas_amount.into(),
            gas_price_per_unit: gas_price_per_unit.into(),
            expiry_time_secs: request.options.expiry_time_secs,
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

    let (account_identifier_signers, unsigned_txn) = if request.signed {
        let signed_txn: SignedTransaction = decode_bcs(&request.transaction, "SignedTransaction")?;
        let mut account_identifier_signers: Vec<_> = vec![signed_txn.sender().into()];

        for account in signed_txn.authenticator().secondary_signer_addreses() {
            account_identifier_signers.push(account.into())
        }

        (
            Some(account_identifier_signers),
            signed_txn.into_raw_transaction(),
        )
    } else {
        let unsigned_txn: RawTransaction = decode_bcs(&request.transaction, "UnsignedTransaction")?;
        (None, unsigned_txn)
    };
    let sender = unsigned_txn.sender();

    // This is messy, but all we can do
    let operations = match unsigned_txn.into_payload() {
        TransactionPayload::EntryFunction(inner) => {
            let (module, function_name, type_args, args) = inner.into_inner();

            let module_name = Identifier::from(module.name());
            if AccountAddress::ONE == *module.address()
                && coin_module_identifier() == module_name
                && transfer_function_identifier() == function_name
            {
                parse_transfer_operation(sender, &type_args, &args)?
            } else if AccountAddress::ONE == *module.address()
                && account_module_identifier() == module_name
                && transfer_function_identifier() == function_name
            {
                parse_account_transfer_operation(sender, &type_args, &args)?
            } else if AccountAddress::ONE == *module.address()
                && account_module_identifier() == module_name
                && create_account_function_identifier() == function_name
            {
                parse_create_account_operation(sender, &type_args, &args)?
            } else if AccountAddress::ONE == *module.address()
                && stake_module_identifier() == module_name
                && set_operator_function_identifier() == function_name
            {
                parse_set_operator_operation(sender, &type_args, &args)?
            } else {
                return Err(ApiError::TransactionParseError(Some(format!(
                    "Unsupported entry function type {:x}::{}::{}",
                    module.address(),
                    module_name,
                    function_name
                ))));
            }
        }
        payload => {
            return Err(ApiError::TransactionParseError(Some(format!(
                "Unsupported transaction payload type {:?}",
                payload
            ))))
        }
    };

    Ok(ConstructionParseResponse {
        operations,
        account_identifier_signers,
    })
}

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
        Err(ApiError::InvalidOperations)
    }
}

fn parse_transfer_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    let mut operations = Vec::new();

    // Check coin is the native coin
    if let Some(TypeTag::Struct(StructTag {
        address,
        module,
        name,
        type_params,
    })) = type_args.first()
    {
        // Currency must be the native coin for now
        if *address != AccountAddress::ONE
            || *module != aptos_coin_module_identifier()
            || *name != aptos_coin_resource_identifier()
            || !type_params.is_empty()
        {
            return Err(ApiError::TransactionParseError(Some(format!(
                "Invalid coin for transfer {:x}::{}::{}",
                address, module, name
            ))));
        }
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No coin type in transfer".to_string(),
        )));
    };

    // Retrieve the args for the operations

    let receiver: AccountAddress = if let Some(receiver) = args.get(0) {
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

    operations.push(Operation::withdraw(0, None, sender, native_coin(), amount));
    operations.push(Operation::deposit(1, None, receiver, native_coin(), amount));
    Ok(operations)
}

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

    let receiver: AccountAddress = if let Some(receiver) = args.get(0) {
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

    operations.push(Operation::withdraw(0, None, sender, native_coin(), amount));
    operations.push(Operation::deposit(1, None, receiver, native_coin(), amount));
    Ok(operations)
}

fn parse_set_operator_operation(
    sender: AccountAddress,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
) -> ApiResult<Vec<Operation>> {
    // There are no typeargs for create account
    if !type_args.is_empty() {
        return Err(ApiError::TransactionParseError(Some(format!(
            "Set operator should not have type arguments: {:?}",
            type_args
        ))));
    }

    // Set operator
    if let Some(encoded_operator) = args.first() {
        let operator: AccountAddress = bcs::from_bytes(encoded_operator)?;

        Ok(vec![Operation::set_operator(0, None, sender, operator)])
    } else {
        Err(ApiError::InvalidOperations)
    }
}

/// Construction payloads command (OFFLINE)
///
/// Constructs payloads for given known operations
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionpayloads)
async fn construction_payloads(
    request: ConstructionPayloadsRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionPayloadsResponse> {
    debug!("/construction/payloads {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    // Retrieve the real operation we're doing
    let operation = InternalOperation::extract(&request.operations)?;
    let metadata = if let Some(ref metadata) = request.metadata {
        metadata
    } else {
        return Err(ApiError::MissingPayloadMetadata);
    };

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

    let signing_message = hex::encode(signing_message(&unsigned_transaction));
    let payload = SigningPayload {
        account_identifier: AccountIdentifier::from(sender),
        hex_bytes: signing_message,
        signature_type: SignatureType::Ed25519,
    };

    // Transaction is both the unsigned transaction and the payload
    Ok(ConstructionPayloadsResponse {
        unsigned_transaction: encode_bcs(&unsigned_transaction)?,
        payloads: vec![payload],
    })
}

/// Construction preprocess command (OFFLINE)
///
/// This creates the request needed to fetch metadata
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionpreprocess)
async fn construction_preprocess(
    request: ConstructionPreprocessRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionPreprocessResponse> {
    debug!("/construction/preprocess {:?}", request);
    check_network(request.network_identifier, &server_context)?;

    let internal_operation = InternalOperation::extract(&request.operations)?;
    let required_public_keys = vec![internal_operation.sender().into()];

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

    let txn: SignedTransaction = decode_bcs(&request.signed_transaction, "SignedTransaction")?;
    let response = rest_client.submit(&txn).await?;
    Ok(ConstructionSubmitResponse {
        transaction_identifier: response.inner().hash.into(),
    })
}

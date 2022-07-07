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
        is_native_coin, native_coin, with_context,
    },
    error::{ApiError, ApiResult},
    types::{InternalOperation, *},
    RosettaContext,
};
use aptos_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    hash::CryptoHash,
    signing_message,
};
use aptos_logger::debug;
use aptos_sdk::{
    move_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    },
    transaction_builder::TransactionFactory,
};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{
        authenticator::AuthenticationKey, RawTransaction, SignedTransaction,
        Transaction::UserTransaction, TransactionPayload,
    },
};
use std::str::FromStr;
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
    let address = AuthenticationKey::ed25519(&public_key)
        .derived_address()
        .to_string();

    let account_identifier = Some(AccountIdentifier {
        address,
        sub_account: None,
    });

    Ok(ConstructionDeriveResponse { account_identifier })
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

    let signed_transaction = decode_bcs(&request.signed_transaction, "SignedTransaction")?;
    let hash = UserTransaction(signed_transaction).hash().to_hex();

    Ok(TransactionIdentifierResponse {
        transaction_identifier: TransactionIdentifier { hash },
    })
}

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
    let response = get_account(rest_client, address).await?;

    // Ensure this network really is the one we expect it to be
    if server_context.chain_id.id() != response.state().chain_id {
        return Err(ApiError::ChainIdMismatch);
    }

    // TODO: Suggest fees?
    Ok(ConstructionMetadataResponse {
        metadata: ConstructionMetadata {
            sequence_number: response.inner().sequence_number,
            max_gas: request.options.max_gas,
            gas_price_per_unit: request.options.gas_price_per_unit,
        },
        suggested_fee: None,
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
    // TODO: Support write set for genesis?
    let operations = match unsigned_txn.into_payload() {
        TransactionPayload::ScriptFunction(inner) => {
            let (module, script_name, type_args, args) = inner.into_inner();

            if AccountAddress::ONE == *module.address()
                && coin_identifier() == Identifier::from(module.name())
                && transfer_identifier() == script_name
            {
                parse_transfer_operation(sender, &type_args, &args)?
            } else if AccountAddress::ONE == *module.address()
                && account_identifier() == Identifier::from(module.name())
                && create_account_identifier() == script_name
            {
                parse_create_account_operation(sender, &type_args, &args)?
            } else {
                return Err(ApiError::TransactionParseError(Some(
                    "Unsupported operation type",
                )));
            }
        }
        _ => {
            return Err(ApiError::TransactionParseError(Some(
                "Unsupported transaction type",
            )))
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
        return Err(ApiError::TransactionParseError(Some(
            "Create account should not have type arguments",
        )));
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
            || *module != test_coin_identifier()
            || *name != test_coin_identifier()
            || !type_params.is_empty()
        {
            return Err(ApiError::TransactionParseError(Some(
                "Invalid coin for transfer",
            )));
        }
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No coin type in transfer",
        )));
    };

    // Retrieve the args for the operations

    let receiver: AccountAddress = if let Some(receiver) = args.get(0) {
        bcs::from_bytes(receiver)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No receiver in transfer",
        )));
    };
    let amount: u64 = if let Some(amount) = args.get(1) {
        bcs::from_bytes(amount)?
    } else {
        return Err(ApiError::TransactionParseError(Some(
            "No amount in transfer",
        )));
    };

    operations.push(Operation::withdraw(0, None, sender, native_coin(), amount));
    operations.push(Operation::deposit(1, None, receiver, native_coin(), amount));
    Ok(operations)
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
    let (txn_payload, sender) = match operation {
        InternalOperation::CreateAccount(create_account) => (
            aptos_stdlib::encode_account_create_account(create_account.new_account),
            create_account.sender,
        ),
        InternalOperation::Transfer(transfer) => {
            is_native_coin(&transfer.currency)?;
            (
                aptos_stdlib::encode_test_coin_transfer(transfer.receiver, transfer.amount),
                transfer.sender,
            )
        }
    };

    // Build the transaction and make it ready for signing
    let transaction_factory = TransactionFactory::new(server_context.chain_id)
        .with_gas_unit_price(metadata.gas_price_per_unit)
        .with_max_gas_amount(metadata.max_gas);
    let sequence_number = metadata.sequence_number;
    let unsigned_transaction = transaction_factory
        .payload(txn_payload)
        .sender(sender)
        .sequence_number(sequence_number)
        .build();

    let signing_message = hex::encode(signing_message(&unsigned_transaction));
    let payload = SigningPayload {
        address: None,
        account_identifier: Some(AccountIdentifier::from(sender)),
        hex_bytes: signing_message,
        signature_type: Some(SignatureType::Ed25519),
    };

    // Transaction is both the unsigned transaction and the payload
    Ok(ConstructionPayloadsResponse {
        unsigned_transaction: encode_bcs(&unsigned_transaction)?,
        payloads: vec![payload],
    })
}

const DEFAULT_GAS_PRICE_PER_UNIT: u64 = 1;
const DEFAULT_MAX_GAS_PRICE: u64 = 10000;

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

    // Ensure that the max fee is only in the native coin
    let max_gas = if let Some(max_fees) = request.max_fee {
        if max_fees.len() != 1 {
            return Err(ApiError::InvalidMaxGasFees);
        }
        let max_fee = max_fees.first().unwrap();
        is_native_coin(&max_fee.currency)?;
        u64::from_str(&max_fee.value)?
    } else {
        DEFAULT_MAX_GAS_PRICE
    };

    // Let's not accept fractions, as we don't support it
    let gas_price_per_unit = if let Some(fee_multiplier) = request.suggested_fee_multiplier {
        if fee_multiplier != (fee_multiplier as u32) as f64 {
            return Err(ApiError::InvalidGasMultiplier);
        }

        fee_multiplier as u64
    } else {
        DEFAULT_GAS_PRICE_PER_UNIT
    };

    let internal_operation = InternalOperation::extract(&request.operations)?;
    let required_public_keys = vec![internal_operation.sender().into()];

    Ok(ConstructionPreprocessResponse {
        options: Some(MetadataOptions {
            internal_operation,
            max_gas,
            gas_price_per_unit,
        }),
        required_public_keys: Some(required_public_keys),
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
        transaction_identifier: TransactionIdentifier {
            hash: response.into_inner().hash.to_string(),
        },
    })
}

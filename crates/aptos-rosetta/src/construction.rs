// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        check_network, decode_bcs, decode_key, encode_bcs, get_account, handle_request,
        with_context,
    },
    error::{ApiError, ApiResult},
    types::{InternalOperation, *},
    RosettaContext,
};
use aptos_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    hash::CryptoHash,
    signing_message, ValidCryptoMaterialStringExt,
};
use aptos_logger::debug;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::transaction::{
    authenticator::AuthenticationKey, RawTransaction, SignedTransaction,
    Transaction::UserTransaction,
};
use std::{convert::TryFrom, str::FromStr};
use warp::Filter;

pub fn routes(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(
            warp::path!("construction" / "combine")
                .and(warp::body::json())
                .and(with_context(server_context.clone()))
                .and_then(handle_request(construction_combine)),
        )
        .or(warp::path!("construction" / "derive")
            .and(warp::body::json())
            .and(with_context(server_context.clone()))
            .and_then(handle_request(construction_derive)))
        .or(warp::path!("construction" / "hash")
            .and(warp::body::json())
            .and(with_context(server_context.clone()))
            .and_then(handle_request(construction_hash)))
        .or(warp::path!("construction" / "metadata")
            .and(warp::body::json())
            .and(with_context(server_context.clone()))
            .and_then(handle_request(construction_metadata)))
        .or(warp::path!("construction" / "parse")
            .and(warp::body::json())
            .and(with_context(server_context.clone()))
            .and_then(handle_request(construction_parse)))
        .or(warp::path!("construction" / "payloads")
            .and(warp::body::json())
            .and(with_context(server_context.clone()))
            .and_then(handle_request(construction_payloads)))
        .or(warp::path!("construction" / "preprocess")
            .and(warp::body::json())
            .and(with_context(server_context.clone()))
            .and_then(handle_request(construction_preprocess)))
        .or(warp::path!("construction" / "submit")
            .and(warp::body::json())
            .and(with_context(server_context))
            .and_then(handle_request(construction_submit)))
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
    debug!("/construction/combine");
    check_network(request.network_identifier, &server_context)?;

    let bytes = hex::decode(request.unsigned_transaction)?;
    let raw_txn: RawTransaction = bcs::from_bytes(&bytes)?;

    // Single signer only supported for now
    // TODO: Support multi-agent / multi-signer?
    if request.signatures.len() != 1 {
        return Err(ApiError::BadSignatureCount);
    }

    let signature = &request.signatures[0];

    if signature.signature_type != SignatureType::Ed25519
        || signature.public_key.curve_type != CurveType::Edwards25519
    {
        return Err(ApiError::BadSignatureType);
    }

    let public_key: Ed25519PublicKey =
        decode_key(&signature.public_key.hex_bytes, "Ed25519PublicKey")?;
    let signature: Ed25519Signature = decode_key(&signature.hex_bytes, "Ed25519Signature")?;

    let signed_txn = SignedTransaction::new(raw_txn, public_key, signature);

    Ok(ConstructionCombineResponse {
        signed_transaction: encode_bcs(&signed_txn)?,
    })
}

/// Construction derive command (OFFLINE)
///
/// Derive account address from Public key
///
/// TODO: What if the public key changes?
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionderive)
async fn construction_derive(
    request: ConstructionDeriveRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionDeriveResponse> {
    debug!("/construction/derive");
    check_network(request.network_identifier, &server_context)?;

    let public_key = Ed25519PublicKey::from_encoded_string(&request.public_key.hex_bytes)
        .map_err(|_| ApiError::deserialization_failed("Ed25519PublicKey"))?;
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
/// Hash a transaction to get it's identifier
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionhash)
async fn construction_hash(
    request: ConstructionHashRequest,
    server_context: RosettaContext,
) -> ApiResult<TransactionIdentifierResponse> {
    debug!("/construction/hash");
    check_network(request.network_identifier, &server_context)?;

    let signed_bytes = hex::decode(&request.signed_transaction)?;
    let signed_transaction: SignedTransaction = bcs::from_bytes(&signed_bytes)
        .map_err(|_| ApiError::deserialization_failed("SignedTransaction"))?;
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
    debug!("/construction/metadata");
    check_network(request.network_identifier, &server_context)?;

    let rest_client = server_context.rest_client()?;
    let address = request.options.internal_operation.sender();
    let response = get_account(rest_client, address).await?;

    // Ensure this network really is the one we expect it to be
    if server_context.chain_id.id() != response.state().chain_id {
        return Err(ApiError::BadNetwork);
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
/// Parses operations from a transaction
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionparse)
async fn construction_parse(
    request: ConstructionParseRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionParseResponse> {
    debug!("/construction/parse");
    check_network(request.network_identifier, &server_context)?;

    let (account_identifier_signers, _raw_txn) = if request.signed {
        let signed_txn: SignedTransaction = decode_bcs(&request.transaction, "SignedTransaction")?;
        let mut account_identifier_signers: Vec<_> = signed_txn
            .authenticator()
            .secondary_signer_addreses()
            .into_iter()
            .map(AccountIdentifier::from)
            .collect();
        let raw_txn = signed_txn.into_raw_transaction();
        account_identifier_signers.push(raw_txn.sender().into());

        (Some(account_identifier_signers), raw_txn)
    } else {
        let raw_txn: RawTransaction = decode_bcs(&request.transaction, "RawTransaction")?;
        (None, raw_txn)
    };

    // TODO: Convert operations
    Ok(ConstructionParseResponse {
        operations: vec![],
        account_identifier_signers,
    })
}

/// Construction payloads command (OFFLINE)
///
/// TODO
///
/// [API Spec](https://www.rosetta-api.org/docs/ConstructionApi.html#constructionpayloads)
async fn construction_payloads(
    request: ConstructionPayloadsRequest,
    server_context: RosettaContext,
) -> ApiResult<ConstructionPayloadsResponse> {
    debug!("/construction/payloads");
    check_network(request.network_identifier, &server_context)?;

    let transfer = Transfer::extract_transfer(&request.operations)?;
    let metadata = if let Some(ref metadata) = request.metadata {
        metadata
    } else {
        return Err(ApiError::BadTransactionPayload);
    };

    // Encode the transfer operation
    let txn_payload = aptos_stdlib::encode_test_coin_transfer(transfer.receiver, transfer.amount);
    let transaction_factory = TransactionFactory::new(server_context.chain_id)
        .with_gas_unit_price(metadata.gas_price_per_unit)
        .with_max_gas_amount(metadata.max_gas);
    let sequence_number = metadata.sequence_number;
    let raw_txn = transaction_factory
        .payload(txn_payload)
        .sender(transfer.sender)
        .sequence_number(sequence_number + 1)
        .build();

    let txn_bytes = signing_message(&raw_txn);
    let hex_bytes = hex::encode(txn_bytes);
    let payload = SigningPayload {
        address: None,
        account_identifier: Some(AccountIdentifier::from(transfer.sender)),
        hex_bytes: hex_bytes.clone(),
        signature_type: Some(SignatureType::Ed25519),
    };

    // Transaction is both the unsigned transaction and the payload
    Ok(ConstructionPayloadsResponse {
        unsigned_transaction: hex_bytes,
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
    debug!("/construction/preprocess");
    check_network(request.network_identifier, &server_context)?;

    // Ensure that the max fee is only in the native coin
    let max_gas = if let Some(max_fees) = request.max_fee {
        if max_fees.len() != 1 {
            return Err(ApiError::BadTransactionPayload);
        }
        let max_fee = max_fees.first().unwrap();
        let _ = SupportedCurrencies::try_from(&max_fee.currency)?;
        u64::from_str(&max_fee.value)?
    } else {
        DEFAULT_MAX_GAS_PRICE
    };

    // Let's not accept fractions, as we don't support it
    let gas_price_per_unit = if let Some(fee_multiplier) = request.suggested_fee_multiplier {
        if fee_multiplier != (fee_multiplier as u64) as f64 {
            return Err(ApiError::BadTransactionPayload);
        }

        fee_multiplier as u64
    } else {
        DEFAULT_GAS_PRICE_PER_UNIT
    };

    Ok(ConstructionPreprocessResponse {
        options: Some(MetadataOptions {
            // We only accept P2P transactions for now
            internal_operation: InternalOperation::extract_transfer(&request.operations)?,
            max_gas,
            gas_price_per_unit,
        }),
        required_public_keys: None,
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
    debug!("/construction/submit");
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

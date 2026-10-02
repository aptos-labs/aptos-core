// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Characterization of the OFFLINE construction endpoints, driven through their
//! warp routes (true wire behavior).
//!
//! The centerpiece is a full offline round trip for an APT transfer:
//! preprocess -> (hand-built metadata) -> payloads -> parse -> sign -> combine
//! -> hash -> parse(signed).  This exercises the APT transfer dispatch, the
//! `InternalOperation` round-trip, payload building, and both parse directions.

use crate::{
    common::native_coin,
    types::{
        AccountIdentifier, ConstructionCombineRequest, ConstructionCombineResponse,
        ConstructionHashRequest, ConstructionMetadata, ConstructionParseRequest,
        ConstructionParseResponse, ConstructionPayloadsRequest, ConstructionPayloadsResponse,
        ConstructionPreprocessRequest, ConstructionPreprocessResponse, ConstructionSubmitResponse,
        InternalOperation, NetworkIdentifier, Operation, PreprocessMetadata, PublicKey, Signature,
        SignatureType,
    },
    RosettaContext,
};
use aptos_crypto::{
    ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform, ValidCryptoMaterialStringExt,
};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{authenticator::AuthenticationKey, RawTransaction, SignedTransaction},
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashSet;
use warp::Filter;

async fn offline_context() -> RosettaContext {
    RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await
}

fn net_id() -> NetworkIdentifier {
    NetworkIdentifier {
        blockchain: "aptos".to_string(),
        network: ChainId::test().to_string(),
    }
}

/// POST a JSON body to a warp filter and deserialize the 200 response.
async fn post_ok<Req, Resp, F>(route: F, path: &str, body: &Req) -> Resp
where
    Req: Serialize,
    Resp: DeserializeOwned,
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: warp::Reply,
{
    let response = warp::test::request()
        .method("POST")
        .path(path)
        .json(body)
        .reply(&route)
        .await;
    assert_eq!(response.status(), 200, "body: {:?}", response.body());
    serde_json::from_slice(response.body()).expect("valid response json")
}

fn transfer_operations(
    sender: AccountAddress,
    receiver: AccountAddress,
    amount: u64,
) -> Vec<Operation> {
    vec![
        Operation::withdraw(
            0,
            None,
            AccountIdentifier::base_account(sender),
            native_coin(),
            amount,
        ),
        Operation::deposit(
            1,
            None,
            AccountIdentifier::base_account(receiver),
            native_coin(),
            amount,
        ),
    ]
}

#[tokio::test]
async fn apt_transfer_full_offline_round_trip() {
    let context = offline_context().await;

    // A deterministic-enough signer (random key; the round trip verifies internal
    // consistency, not a fixed golden address).
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = PublicKey::try_from(private_key.public_key()).unwrap();
    let sender = AuthenticationKey::ed25519(&private_key.public_key()).account_address();
    let receiver = AccountAddress::from_hex_literal("0x1234").unwrap();
    let amount = 1_000u64;
    let operations = transfer_operations(sender, receiver, amount);

    // 1) preprocess -> options carrying the resolved Transfer InternalOperation.
    let preprocess: ConstructionPreprocessResponse = post_ok(
        crate::construction::preprocess_route(context.clone()),
        "/construction/preprocess",
        &ConstructionPreprocessRequest {
            network_identifier: net_id(),
            operations: operations.clone(),
            metadata: Some(PreprocessMetadata {
                expiry_time_secs: None,
                sequence_number: None,
                max_gas_amount: None,
                gas_price: None,
                public_keys: Some(vec![public_key.clone()]),
                gas_price_multiplier: None,
                gas_price_priority: None,
            }),
        },
    )
    .await;

    match &preprocess.options.internal_operation {
        InternalOperation::Transfer(transfer) => {
            assert_eq!(transfer.sender, sender);
            assert_eq!(transfer.receiver, receiver);
            assert_eq!(transfer.amount.0, amount);
            assert_eq!(transfer.currency, native_coin());
        },
        other => panic!("expected Transfer internal operation, got {:?}", other),
    }
    assert_eq!(preprocess.required_public_keys, vec![
        AccountIdentifier::base_account(sender)
    ]);

    // 2) metadata is an ONLINE call; build it by hand so payloads can run offline.
    let metadata = ConstructionMetadata {
        sequence_number: 0.into(),
        max_gas_amount: 100_000.into(),
        gas_price_per_unit: 100.into(),
        expiry_time_secs: Some(1_000_000.into()),
        internal_operation: preprocess.options.internal_operation.clone(),
    };

    // 3) payloads -> unsigned transaction + signing payloads.
    let payloads: ConstructionPayloadsResponse = post_ok(
        crate::construction::payloads_route(context.clone()),
        "/construction/payloads",
        &ConstructionPayloadsRequest {
            network_identifier: net_id(),
            operations: operations.clone(),
            metadata: Some(metadata),
            public_keys: Some(vec![public_key.clone()]),
        },
    )
    .await;
    assert_eq!(payloads.payloads.len(), 1);
    assert_eq!(
        payloads.payloads[0].account_identifier,
        AccountIdentifier::base_account(sender)
    );

    // 4) parse the UNSIGNED transaction -> operations match, no signers.
    let parsed_unsigned: ConstructionParseResponse = post_ok(
        crate::construction::parse_route(context.clone()),
        "/construction/parse",
        &ConstructionParseRequest {
            network_identifier: net_id(),
            signed: false,
            transaction: payloads.unsigned_transaction.clone(),
        },
    )
    .await;
    assert!(parsed_unsigned.account_identifier_signers.is_none());
    assert_transfer_operations(&parsed_unsigned.operations, sender, receiver, amount);

    // 5) sign the payload exactly as RosettaClient does.
    let unsigned_txn: RawTransaction =
        bcs::from_bytes(&hex::decode(&payloads.unsigned_transaction).unwrap()).unwrap();
    let signing_message = hex::encode(unsigned_txn.signing_message().unwrap());
    assert_eq!(signing_message, payloads.payloads[0].hex_bytes);
    let txn_signature = private_key.sign(&unsigned_txn).unwrap();
    let signature = Signature {
        signing_payload: payloads.payloads[0].clone(),
        public_key: public_key.clone(),
        signature_type: SignatureType::Ed25519,
        hex_bytes: txn_signature.to_encoded_string().unwrap(),
    };

    // 6) combine -> signed transaction.
    let combined: ConstructionCombineResponse = post_ok(
        crate::construction::combine_route(context.clone()),
        "/construction/combine",
        &ConstructionCombineRequest {
            network_identifier: net_id(),
            unsigned_transaction: payloads.unsigned_transaction.clone(),
            signatures: vec![signature],
        },
    )
    .await;

    // The combined blob must deserialize as a SignedTransaction with our signer.
    let signed_txn: SignedTransaction =
        bcs::from_bytes(&hex::decode(&combined.signed_transaction).unwrap()).unwrap();
    assert_eq!(signed_txn.sender(), sender);

    // 7) hash -> a 0x-prefixed 32-byte hex string.
    let hashed: ConstructionSubmitResponse = post_ok(
        crate::construction::hash_route(context.clone()),
        "/construction/hash",
        &ConstructionHashRequest {
            network_identifier: net_id(),
            signed_transaction: combined.signed_transaction.clone(),
        },
    )
    .await;
    // CHARACTERIZATION: transaction identifiers are bare lowercase hex with NO
    // `0x` prefix (via `to_hex_lower` == `format!("{:x}")`).  See
    // docs/SPEC_DEVIATIONS.md §14.
    let hash = hashed.transaction_identifier.hash;
    assert!(
        !hash.starts_with("0x"),
        "hash unexpectedly 0x-prefixed: {}",
        hash
    );
    assert_eq!(hash.len(), 64, "hash was {}", hash);
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "hash not lowercase hex: {}",
        hash
    );

    // 8) parse the SIGNED transaction -> operations match, signer present.
    let parsed_signed: ConstructionParseResponse = post_ok(
        crate::construction::parse_route(context.clone()),
        "/construction/parse",
        &ConstructionParseRequest {
            network_identifier: net_id(),
            signed: true,
            transaction: combined.signed_transaction.clone(),
        },
    )
    .await;
    assert_eq!(
        parsed_signed.account_identifier_signers,
        Some(vec![AccountIdentifier::base_account(sender)])
    );
    assert_transfer_operations(&parsed_signed.operations, sender, receiver, amount);
}

/// A transfer parses back to a withdraw (from sender, -amount) and a deposit
/// (to receiver, +amount), both in APT.
fn assert_transfer_operations(
    operations: &[Operation],
    sender: AccountAddress,
    receiver: AccountAddress,
    amount: u64,
) {
    assert_eq!(operations.len(), 2, "operations: {:?}", operations);

    let withdraw = operations
        .iter()
        .find(|op| op.operation_type == "withdraw")
        .expect("withdraw operation present");
    assert_eq!(
        withdraw.account,
        Some(AccountIdentifier::base_account(sender))
    );
    let withdraw_amount = withdraw.amount.as_ref().unwrap();
    assert_eq!(withdraw_amount.value, format!("-{}", amount));
    assert_eq!(withdraw_amount.currency, native_coin());

    let deposit = operations
        .iter()
        .find(|op| op.operation_type == "deposit")
        .expect("deposit operation present");
    assert_eq!(
        deposit.account,
        Some(AccountIdentifier::base_account(receiver))
    );
    let deposit_amount = deposit.amount.as_ref().unwrap();
    assert_eq!(deposit_amount.value, amount.to_string());
    assert_eq!(deposit_amount.currency, native_coin());
}

#[tokio::test]
async fn combine_rejects_multiple_signatures() {
    let context = offline_context().await;

    // Build a minimal valid unsigned transaction to combine against.
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = PublicKey::try_from(private_key.public_key()).unwrap();
    let sender = AuthenticationKey::ed25519(&private_key.public_key()).account_address();
    let receiver = AccountAddress::from_hex_literal("0x1234").unwrap();
    let operations = transfer_operations(sender, receiver, 1);

    let preprocess: ConstructionPreprocessResponse = post_ok(
        crate::construction::preprocess_route(context.clone()),
        "/construction/preprocess",
        &ConstructionPreprocessRequest {
            network_identifier: net_id(),
            operations: operations.clone(),
            metadata: Some(PreprocessMetadata {
                expiry_time_secs: None,
                sequence_number: None,
                max_gas_amount: None,
                gas_price: None,
                public_keys: Some(vec![public_key.clone()]),
                gas_price_multiplier: None,
                gas_price_priority: None,
            }),
        },
    )
    .await;
    let metadata = ConstructionMetadata {
        sequence_number: 0.into(),
        max_gas_amount: 100_000.into(),
        gas_price_per_unit: 100.into(),
        expiry_time_secs: Some(1_000_000.into()),
        internal_operation: preprocess.options.internal_operation,
    };
    let payloads: ConstructionPayloadsResponse = post_ok(
        crate::construction::payloads_route(context.clone()),
        "/construction/payloads",
        &ConstructionPayloadsRequest {
            network_identifier: net_id(),
            operations,
            metadata: Some(metadata),
            public_keys: Some(vec![public_key.clone()]),
        },
    )
    .await;

    let unsigned_txn: RawTransaction =
        bcs::from_bytes(&hex::decode(&payloads.unsigned_transaction).unwrap()).unwrap();
    let sig = private_key.sign(&unsigned_txn).unwrap();
    let signature = Signature {
        signing_payload: payloads.payloads[0].clone(),
        public_key,
        signature_type: SignatureType::Ed25519,
        hex_bytes: sig.to_encoded_string().unwrap(),
    };

    // Two signatures -> UnsupportedSignatureCount (single-signer only, see §10).
    let response = warp::test::request()
        .method("POST")
        .path("/construction/combine")
        .json(&ConstructionCombineRequest {
            network_identifier: net_id(),
            unsigned_transaction: payloads.unsigned_transaction.clone(),
            signatures: vec![signature.clone(), signature],
        })
        .reply(&crate::construction::combine_route(context))
        .await;
    assert_eq!(response.status(), 500);
    let body: crate::types::Error = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(
        body.code,
        crate::error::ApiError::UnsupportedSignatureCount(None).code()
    );
}

#[tokio::test]
async fn create_account_offline_round_trip() {
    let context = offline_context().await;
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = PublicKey::try_from(private_key.public_key()).unwrap();
    let sender = AuthenticationKey::ed25519(&private_key.public_key()).account_address();
    let new_account = AccountAddress::from_hex_literal("0x1234").unwrap();
    let operations = vec![Operation::create_account(0, None, new_account, sender)];

    let preprocess: ConstructionPreprocessResponse = post_ok(
        crate::construction::preprocess_route(context.clone()),
        "/construction/preprocess",
        &ConstructionPreprocessRequest {
            network_identifier: net_id(),
            operations: operations.clone(),
            metadata: Some(PreprocessMetadata {
                expiry_time_secs: None,
                sequence_number: None,
                max_gas_amount: None,
                gas_price: None,
                public_keys: Some(vec![public_key.clone()]),
                gas_price_multiplier: None,
                gas_price_priority: None,
            }),
        },
    )
    .await;
    match &preprocess.options.internal_operation {
        InternalOperation::CreateAccount(create) => {
            assert_eq!(create.sender, sender);
            assert_eq!(create.new_account, new_account);
        },
        other => panic!("expected CreateAccount, got {:?}", other),
    }

    let metadata = ConstructionMetadata {
        sequence_number: 0.into(),
        max_gas_amount: 100_000.into(),
        gas_price_per_unit: 100.into(),
        expiry_time_secs: Some(1_000_000.into()),
        internal_operation: preprocess.options.internal_operation.clone(),
    };
    let payloads: ConstructionPayloadsResponse = post_ok(
        crate::construction::payloads_route(context.clone()),
        "/construction/payloads",
        &ConstructionPayloadsRequest {
            network_identifier: net_id(),
            operations: operations.clone(),
            metadata: Some(metadata),
            public_keys: Some(vec![public_key]),
        },
    )
    .await;

    let parsed: ConstructionParseResponse = post_ok(
        crate::construction::parse_route(context),
        "/construction/parse",
        &ConstructionParseRequest {
            network_identifier: net_id(),
            signed: false,
            transaction: payloads.unsigned_transaction,
        },
    )
    .await;
    assert_eq!(parsed.operations.len(), 1);
    assert_eq!(parsed.operations[0].operation_type, "create_account");
    assert_eq!(
        parsed.operations[0].account,
        Some(AccountIdentifier::base_account(new_account))
    );
}

#[tokio::test]
async fn derive_returns_ed25519_authkey_address() {
    let context = offline_context().await;
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = PublicKey::try_from(private_key.public_key()).unwrap();
    let expected = AuthenticationKey::ed25519(&private_key.public_key()).account_address();

    let response: crate::types::ConstructionDeriveResponse = post_ok(
        crate::construction::derive_route(context),
        "/construction/derive",
        &crate::types::ConstructionDeriveRequest {
            network_identifier: net_id(),
            public_key,
        },
    )
    .await;
    assert_eq!(
        response.account_identifier,
        AccountIdentifier::base_account(expected)
    );
}

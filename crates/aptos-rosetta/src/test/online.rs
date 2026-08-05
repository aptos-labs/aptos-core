// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Characterization of the ONLINE endpoints (network/status, block,
//! account/balance) driven through their warp routes with a mocked
//! [`NodeClient`].  This proves the Phase 1a seam lets us unit-test the handler
//! orchestration (block boundaries, balance version selection, currency
//! dispatch) with no running node.

use crate::{
    block::BlockRetriever,
    common::{native_coin, BlockHash},
    node_client::{MockNodeClient, NodeClient},
    types::{
        AccountBalanceRequest, AccountBalanceResponse, AccountIdentifier, BlockRequest,
        BlockResponse, NetworkIdentifier, NetworkRequest, NetworkStatusResponse,
        PartialBlockIdentifier,
    },
    RosettaContext,
};
use aptos_crypto::HashValue;
use aptos_rest_client::{
    aptos_api_types::{AptosError, AptosErrorCode, BcsBlock},
    error::{AptosErrorResponse, RestError},
    Response, State,
};
use aptos_types::chain_id::ChainId;
use std::{collections::HashSet, sync::Arc};
use warp::http::StatusCode;

fn net_id() -> NetworkIdentifier {
    NetworkIdentifier {
        blockchain: "aptos".to_string(),
        network: ChainId::test().to_string(),
    }
}

fn ledger_state(block_height: u64, oldest_block_height: u64, version: u64, ts_usecs: u64) -> State {
    State {
        chain_id: ChainId::test().id(),
        epoch: 1,
        version,
        timestamp_usecs: ts_usecs,
        oldest_ledger_version: 0,
        oldest_block_height,
        block_height,
        cursor: None,
        encryption_key: None,
    }
}

fn bcs_block(
    height: u64,
    ts_usecs: u64,
    transactions: Option<Vec<aptos_rest_client::aptos_api_types::TransactionOnChainData>>,
) -> BcsBlock {
    BcsBlock {
        block_height: height,
        block_hash: HashValue::zero(),
        block_timestamp: ts_usecs,
        first_version: height * 10,
        last_version: height * 10 + 5,
        transactions,
    }
}

/// Builds a context whose node client and block retriever share one mock.
async fn context_with(mock: MockNodeClient) -> RosettaContext {
    let node: Arc<dyn NodeClient> = Arc::new(mock);
    let retriever = Arc::new(BlockRetriever::new(100, node.clone()));
    RosettaContext::new(Some(node), ChainId::test(), Some(retriever), HashSet::new()).await
}

fn account_not_found() -> RestError {
    RestError::Api(AptosErrorResponse {
        error: AptosError {
            message: "not found".to_string(),
            error_code: AptosErrorCode::AccountNotFound,
            vm_error_code: None,
        },
        state: None,
        status_code: StatusCode::NOT_FOUND,
    })
}

#[tokio::test]
async fn network_status_reports_current_oldest_and_genesis() {
    let current_height = 7u64;
    let ts_usecs = 1_700_000_000_000_000u64; // well after Y2K

    let mut mock = MockNodeClient::new();
    mock.expect_get_ledger_information().returning(move || {
        let state = ledger_state(current_height, 0, current_height * 10 + 5, ts_usecs);
        Box::pin(async move { Ok(Response::new(state.clone(), state)) })
    });
    // Only the current (non-genesis) block requires a node fetch; oldest = 0 and
    // genesis are hardcoded.
    //
    // `network_status` calls `get_block_info_by_height` three times (genesis,
    // oldest, current), but that helper short-circuits `height == 0` with a
    // hardcoded genesis `BlockInfo` (`block.rs`), and `oldest_block_height` is 0
    // here -- so exactly ONE call reaches the node.  Pinned at 1 so losing that
    // short-circuit (3 fetches instead of 1) fails here instead of silently
    // tripling round trips on every /network/status.
    mock.expect_get_block_by_height_bcs()
        .times(1)
        .returning(move |height, _with_txns| {
            Box::pin(async move {
                Ok(Response::new(
                    bcs_block(height, ts_usecs, None),
                    dummy_state(),
                ))
            })
        });

    let context = context_with(mock).await;
    let response = warp::test::request()
        .method("POST")
        .path("/network/status")
        .json(&NetworkRequest {
            network_identifier: net_id(),
        })
        .reply(&crate::network::status_route(context))
        .await;
    assert_eq!(response.status(), 200, "body {:?}", response.body());
    let body: NetworkStatusResponse = serde_json::from_slice(response.body()).unwrap();

    assert_eq!(body.current_block_identifier.index, current_height);
    assert_eq!(
        body.current_block_identifier.hash,
        BlockHash::new(ChainId::test(), current_height).to_string()
    );
    assert_eq!(body.genesis_block_identifier.index, 0);
    assert_eq!(
        body.genesis_block_identifier.hash,
        BlockHash::new(ChainId::test(), 0).to_string()
    );
    assert_eq!(body.oldest_block_identifier.index, 0);
    // Timestamp is milliseconds (usecs / 1000), see docs/SPEC_DEVIATIONS.md §3.
    assert_eq!(body.current_block_timestamp, ts_usecs / 1000);
}

#[tokio::test]
async fn block_drops_empty_transactions_by_default() {
    let height = 5u64;
    let ts_usecs = 1_700_000_000_000_000u64;

    let mut mock = MockNodeClient::new();
    // Full block (with transactions) for the requested height...
    mock.expect_get_full_block_by_height_bcs()
        .times(1)
        .returning(move |h, _page| {
            Box::pin(async move {
                Ok(Response::new(
                    bcs_block(h, ts_usecs, Some(vec![])),
                    dummy_state(),
                ))
            })
        });
    // ...and the parent block (without transactions) for the identifier.
    mock.expect_get_block_by_height_bcs()
        .times(1)
        .returning(move |h, _with| {
            Box::pin(async move { Ok(Response::new(bcs_block(h, ts_usecs, None), dummy_state())) })
        });

    let context = context_with(mock).await;
    let response = warp::test::request()
        .method("POST")
        .path("/block")
        .json(&BlockRequest {
            network_identifier: net_id(),
            block_identifier: Some(PartialBlockIdentifier::block_index(height)),
            metadata: None,
        })
        .reply(&crate::block::block_route(context))
        .await;
    assert_eq!(response.status(), 200, "body {:?}", response.body());
    let body: BlockResponse = serde_json::from_slice(response.body()).unwrap();

    assert_eq!(body.block.block_identifier.index, height);
    assert_eq!(body.block.parent_block_identifier.index, height - 1);
    assert!(
        body.block.transactions.is_empty(),
        "empty transactions should be dropped by default"
    );
    assert_eq!(body.block.timestamp, ts_usecs / 1000);
}

#[tokio::test]
async fn account_balance_reports_apt_coin_balance() {
    let height = 3u64;
    let ts_usecs = 1_700_000_000_000_000u64;
    let coin_balance = 12_345u64;
    let address = aptos_types::account_address::AccountAddress::from_hex_literal("0xabc").unwrap();

    let mut mock = MockNodeClient::new();
    mock.expect_get_block_by_height_bcs()
        .returning(move |h, _with| {
            Box::pin(async move { Ok(Response::new(bcs_block(h, ts_usecs, None), dummy_state())) })
        });
    // No Account resource -> sequence number falls back to 0 (see §12).
    mock.expect_get_account_resource_at_version_bytes()
        .returning(|_, _, _| Box::pin(async { Err(account_not_found()) }));
    // The coin balance view function returns [balance].
    mock.expect_view_bcs_u64s()
        .returning(move |_req, _version| {
            Box::pin(async move { Ok(Response::new(vec![coin_balance], dummy_state())) })
        });

    let context = context_with(mock).await;
    let response = warp::test::request()
        .method("POST")
        .path("/account/balance")
        .json(&AccountBalanceRequest {
            network_identifier: net_id(),
            account_identifier: AccountIdentifier::base_account(address),
            block_identifier: Some(PartialBlockIdentifier::block_index(height)),
            // Filter to APT only so exactly one balance view runs.
            currencies: Some(vec![native_coin()]),
        })
        .reply(&crate::account::routes(context))
        .await;
    assert_eq!(response.status(), 200, "body {:?}", response.body());
    let body: AccountBalanceResponse = serde_json::from_slice(response.body()).unwrap();

    assert_eq!(body.block_identifier.index, height);
    assert_eq!(body.balances.len(), 1);
    assert_eq!(body.balances[0].value, coin_balance.to_string());
    assert_eq!(body.balances[0].currency, native_coin());
    // Missing Account resource -> sequence number 0.
    assert_eq!(body.metadata.sequence_number.0, 0);
}

#[tokio::test]
async fn construction_submit_returns_transaction_hash() {
    use aptos_crypto::{PrivateKey, SigningKey, Uniform};
    use aptos_types::test_helpers::transaction_test_helpers::get_test_raw_transaction;

    let private_key = aptos_crypto::ed25519::Ed25519PrivateKey::generate_for_testing();
    let sender = aptos_types::transaction::authenticator::AuthenticationKey::ed25519(
        &private_key.public_key(),
    )
    .account_address();
    let raw_txn = get_test_raw_transaction(sender, 0, None, None, Some(100), None);
    let signature = private_key.sign(&raw_txn).unwrap();
    let signed_txn = aptos_types::transaction::SignedTransaction::new(
        raw_txn,
        private_key.public_key(),
        signature,
    );
    let expected_hash = crate::common::to_hex_lower(&signed_txn.committed_hash());
    let signed_hex = hex::encode(bcs::to_bytes(&signed_txn).unwrap());

    let mut mock = MockNodeClient::new();
    mock.expect_submit_bcs()
        .returning(|_txn| Box::pin(async { Ok(Response::new((), dummy_state())) }));

    let context = context_with(mock).await;
    let response = warp::test::request()
        .method("POST")
        .path("/construction/submit")
        .json(&crate::types::ConstructionSubmitRequest {
            network_identifier: net_id(),
            signed_transaction: signed_hex,
        })
        .reply(&crate::construction::submit_route(context))
        .await;
    assert_eq!(response.status(), 200, "body {:?}", response.body());
    let body: crate::types::ConstructionSubmitResponse =
        serde_json::from_slice(response.body()).unwrap();
    assert_eq!(body.transaction_identifier.hash, expected_hash);
}

fn dummy_state() -> State {
    ledger_state(0, 0, 0, 0)
}

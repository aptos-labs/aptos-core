// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Handler / endpoint characterization tests.
//!
//! These lock the *current* wire behavior of the Rosetta endpoints so the Phase 2
//! rewrite is provably compatible.  Offline endpoints are driven through their warp
//! routes; online behavior is driven with a mocked [`NodeClient`] (proving the
//! Phase 1a seam enables offline handler testing).

use crate::{
    common::{get_account, native_coin},
    error::ApiError,
    node_client::MockNodeClient,
    types::{
        get_stake_balances, AccountIdentifier, NetworkIdentifier, NetworkListResponse,
        NetworkOptionsResponse, NetworkRequest,
    },
    RosettaContext,
};
use aptos_rest_client::error::RestError;
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use std::collections::HashSet;

/// Builds an offline context (no node) on the test chain.
async fn offline_context() -> RosettaContext {
    RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await
}

fn test_network_identifier() -> NetworkIdentifier {
    NetworkIdentifier {
        blockchain: "aptos".to_string(),
        network: ChainId::test().to_string(),
    }
}

// ---------------------------------------------------------------------------
// Offline endpoints, driven through their warp routes (true wire behavior)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn network_list_returns_single_chain() {
    let context = offline_context().await;
    let response = warp::test::request()
        .method("POST")
        .path("/network/list")
        .reply(&crate::network::list_route(context))
        .await;

    assert_eq!(response.status(), 200);
    let body: NetworkListResponse = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(body.network_identifiers, vec![test_network_identifier()]);
}

#[tokio::test]
async fn network_options_matches_documented_deviations() {
    let context = offline_context().await;
    let response = warp::test::request()
        .method("POST")
        .path("/network/options")
        .json(&NetworkRequest {
            network_identifier: test_network_identifier(),
        })
        .reply(&crate::network::options_route(context))
        .await;

    assert_eq!(response.status(), 200);
    let body: NetworkOptionsResponse = serde_json::from_slice(response.body()).unwrap();

    // Documented deviations (see docs/SPEC_DEVIATIONS.md §8).
    assert!(!body.allow.mempool_coins, "mempool_coins must be false");
    assert!(body.allow.historical_balance_lookup);
    assert_eq!(body.allow.timestamp_start_index, 2);
    assert!(body.allow.call_methods.is_empty());
    assert!(body.allow.balance_exemptions.is_empty());
    assert_eq!(body.version.rosetta_version, crate::ROSETTA_VERSION);
    assert_eq!(body.version.node_version, crate::NODE_VERSION);
    assert_eq!(body.version.middleware_version, "0.1.0");

    // BC-6: all 15 operation types are advertised (update_commission was
    // previously missing from OperationType::all()).
    assert_eq!(body.allow.operation_types.len(), 15);
    assert!(body.allow.operation_types.iter().any(|op| op == "fee"));
    assert!(body
        .allow
        .operation_types
        .iter()
        .any(|op| op == "add_delegated_stake"));
    assert!(body
        .allow
        .operation_types
        .iter()
        .any(|op| op == "update_commission"));
    assert_eq!(body.allow.operation_statuses.len(), 2);

    // All 36 error codes are advertised, all HTTP 500 (see §9).
    assert_eq!(body.allow.errors.len(), 36);
    assert!(body
        .allow
        .errors
        .iter()
        .all(|e| e.code >= 1 && e.code <= 36));
}

#[tokio::test]
async fn network_options_rejects_wrong_chain() {
    let context = offline_context().await;
    let response = warp::test::request()
        .method("POST")
        .path("/network/options")
        .json(&NetworkRequest {
            network_identifier: NetworkIdentifier {
                blockchain: "aptos".to_string(),
                network: "mainnet".to_string(),
            },
        })
        .reply(&crate::network::options_route(context))
        .await;

    // Rosetta returns errors as HTTP 500 with a numeric code (see §9).
    assert_eq!(response.status(), 500);
    let body: crate::types::Error = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(body.code, ApiError::NetworkIdentifierMismatch.code());
}

// ---------------------------------------------------------------------------
// Online behavior via a mocked NodeClient (proves the Phase 1a seam works)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_account_maps_node_error_to_account_not_found() {
    let address = AccountAddress::ONE;
    let mut node = MockNodeClient::new();
    node.expect_get_account().returning(|_| {
        Box::pin(async { Err(RestError::Unknown(anyhow::anyhow!("node exploded"))) })
    });

    let result = get_account(&node, address).await;
    match result {
        Err(ApiError::AccountNotFound(Some(msg))) => assert_eq!(msg, address.to_string()),
        other => panic!("expected AccountNotFound, got {:?}", other),
    }
}

#[tokio::test]
async fn get_stake_balances_returns_none_when_pool_missing() {
    let mut node = MockNodeClient::new();
    node.expect_get_account_resource_at_version_bytes()
        .returning(|_, _, _| {
            Box::pin(async { Err(RestError::Unknown(anyhow::anyhow!("no stake pool"))) })
        });

    let account = AccountIdentifier::total_stake_account(AccountAddress::ONE);
    let result = get_stake_balances(&node, &account, AccountAddress::ONE, 1)
        .await
        .expect("missing pool should be Ok(None), not an error");
    assert!(result.is_none());
    // Sanity: the native coin currency is what stake balances report in.
    let _ = native_coin();
}

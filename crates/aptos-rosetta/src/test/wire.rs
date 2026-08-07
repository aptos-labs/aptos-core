// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the *real* [`RestNodeClient`] wire path.
//!
//! Where the mock-based tests validate handler logic against the trait, these
//! stand up a `wiremock` HTTP server, point a real `aptos_rest_client::Client`
//! at it, and confirm that the production `NodeClient` impl serializes requests
//! and parses responses/errors correctly over HTTP.

use crate::{
    error::ApiError,
    node_client::{NodeClient, RestNodeClient},
};
use aptos_rest_client::{error::RestError, Client};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use url::Url;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

/// Adds the Aptos ledger-state headers every REST response carries, so the
/// client can build its `Response<T>` wrapper.
fn with_aptos_headers(template: ResponseTemplate) -> ResponseTemplate {
    template
        .insert_header(
            "X-Aptos-Chain-Id",
            ChainId::test().id().to_string().as_str(),
        )
        .insert_header("X-Aptos-Epoch", "1")
        .insert_header("X-Aptos-Ledger-Version", "100")
        .insert_header("X-Aptos-Ledger-Oldest-Version", "0")
        .insert_header("X-Aptos-Ledger-TimestampUsec", "1700000000000000")
        .insert_header("X-Aptos-Block-Height", "5")
        .insert_header("X-Aptos-Oldest-Block-Height", "0")
}

fn node_client(server: &MockServer) -> RestNodeClient {
    RestNodeClient::new(Client::new(Url::parse(&server.uri()).unwrap()))
}

#[tokio::test]
async fn rest_node_client_parses_gas_estimation_over_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(with_aptos_headers(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deprioritized_gas_estimate": 90,
                "gas_estimate": 100,
                "prioritized_gas_estimate": 150,
            })),
        ))
        .mount(&server)
        .await;

    let node = node_client(&server);
    let estimation = node
        .estimate_gas_price()
        .await
        .expect("gas estimation should parse")
        .into_inner();

    assert_eq!(estimation.gas_estimate, 100);
    assert_eq!(estimation.deprioritized_gas_estimate, Some(90));
    assert_eq!(estimation.prioritized_gas_estimate, Some(150));
}

#[tokio::test]
async fn rest_node_client_propagates_account_not_found_error() {
    let server = MockServer::start().await;
    // A 404 with the node's error JSON shape.
    Mock::given(method("GET"))
        .respond_with(with_aptos_headers(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "Account not found by address(0x1)",
                "error_code": "account_not_found",
            })),
        ))
        .mount(&server)
        .await;

    let node = node_client(&server);
    let result = node
        .get_account_resource_bytes(AccountAddress::ONE, "0x1::account::Account")
        .await;

    // The real client must surface this as a typed RestError...
    let err = result.expect_err("should be an error");
    assert!(
        matches!(&err, RestError::Api(_)),
        "expected RestError::Api, got {:?}",
        err
    );
    // ...which Rosetta then maps to its AccountNotFound (code 18).
    let api_error: ApiError = err.into();
    assert_eq!(api_error.code(), ApiError::AccountNotFound(None).code());
}

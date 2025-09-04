// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use velor_types::jwks::jwk::JWK;
use http::header::COOKIE;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct OpenIDConfiguration {
    issuer: String,
    jwks_uri: String,
}

#[derive(Serialize, Deserialize)]
struct JWKsResponse {
    keys: Vec<serde_json::Value>,
}

/// Given a JWK URL, fetch its JWKs.
///
/// Optionally, if an address is given, send it as the cookie payload.
/// The optional logic is only used in smoke tests, e.g., `jwk_consensus_basic`.
pub async fn fetch_jwks_from_jwks_uri(
    my_addr: Option<AccountAddress>,
    jwks_uri: &str,
) -> Result<Vec<JWK>> {
    let client = reqwest::Client::new();
    let mut request_builder = client.get(jwks_uri);
    if let Some(addr) = my_addr {
        request_builder = request_builder.header(COOKIE, addr.to_hex());
    }
    let JWKsResponse { keys } = request_builder.send().await?.json().await?;
    let jwks = keys.into_iter().map(JWK::from).collect();
    Ok(jwks)
}

/// Given an Open ID configuration URL, fetch its JWK url.
pub async fn fetch_jwks_uri_from_openid_config(config_url: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let OpenIDConfiguration { jwks_uri, .. } = client.get(config_url).send().await?.json().await?;
    Ok(jwks_uri)
}

#[ignore]
#[tokio::test]
async fn test_fetch_real_jwks() {
    let jwks_uri = fetch_jwks_uri_from_openid_config(
        "https://www.facebook.com/.well-known/openid-configuration/",
    )
    .await
    .unwrap();
    let jwks = fetch_jwks_from_jwks_uri(None, jwks_uri.as_str())
        .await
        .unwrap();
    println!("{:?}", jwks);
}

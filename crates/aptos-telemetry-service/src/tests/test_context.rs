// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::clients::humio;
use crate::GCPBigQueryConfig;
use crate::{context::Context, index, validator_cache::PeerSetCache, TelemetryServiceConfig};
use aptos_config::keys::ConfigKey;
use aptos_crypto::{x25519, Uniform};
use aptos_rest_client::aptos_api_types::mime_types;
use rand::SeedableRng;
use reqwest::header::AUTHORIZATION;
use reqwest::Url;
use serde_json::Value;
use warp::http::header::CONTENT_TYPE;
use warp::http::Response;
use warp::hyper::body::Bytes;

pub async fn new_test_context() -> TestContext {
    let mut rng = ::rand::rngs::StdRng::from_seed([0u8; 32]);
    let server_private_key = x25519::PrivateKey::generate(&mut rng);

    let config = &TelemetryServiceConfig {
        address: format!("{}:{}", "127.0.0.1", 80).parse().unwrap(),
        tls_cert_path: None,
        tls_key_path: None,
        trusted_full_node_addresses: HashMap::new(),
        server_private_key: ConfigKey::new(server_private_key),
        jwt_signing_key: "jwt_signing_key".into(),
        update_interval: 60,
        gcp_bq_config: GCPBigQueryConfig {
            project_id: String::from("1"),
            dataset_id: String::from("2"),
            table_id: String::from("3"),
        },
        victoria_metrics_base_url: "".into(),
        victoria_metrics_token: "".into(),
        humio_url: "".into(),
        humio_auth_token: "".into(),
    };
    let humio_client = humio::IngestClient::new(
        Url::parse("http://localhost/").unwrap(),
        config.humio_auth_token.clone(),
    );
    let gcp_bigquery_client = gcp_bigquery_client::Client::with_workload_identity(false)
        .await
        .unwrap();
    let validator_cache = PeerSetCache::new(aptos_infallible::RwLock::new(HashMap::new()));
    let vfn_cache = PeerSetCache::new(aptos_infallible::RwLock::new(HashMap::new()));

    TestContext::new(Context::new(
        config,
        validator_cache,
        vfn_cache,
        Some(gcp_bigquery_client),
        None,
        humio_client,
    ))
}

#[derive(Clone)]
pub struct TestContext {
    expect_status_code: u16,
    pub inner: Context,
    bearer_token: String,
}

impl TestContext {
    pub fn new(context: Context) -> Self {
        Self {
            expect_status_code: 200,
            inner: context,
            bearer_token: "".into(),
        }
    }

    pub fn expect_status_code(&self, status_code: u16) -> Self {
        let mut ret = self.clone();
        ret.expect_status_code = status_code;
        ret
    }

    pub fn with_bearer_auth(&self, token: String) -> Self {
        let mut ret = self.clone();
        ret.bearer_token = token;
        ret
    }

    pub async fn get(&self, path: &str) -> Value {
        self.execute(
            warp::test::request()
                .header(AUTHORIZATION, format!("Bearer {}", self.bearer_token))
                .method("GET")
                .path(path),
        )
        .await
    }

    pub async fn post(&self, path: &str, body: Value) -> Value {
        self.execute(
            warp::test::request()
                .header(AUTHORIZATION, format!("Bearer {}", self.bearer_token))
                .method("POST")
                .path(path)
                .json(&body),
        )
        .await
    }

    pub async fn reply(&self, req: warp::test::RequestBuilder) -> Response<Bytes> {
        req.reply(&index::routes(self.inner.clone())).await
    }

    pub async fn execute(&self, req: warp::test::RequestBuilder) -> Value {
        let resp = self.reply(req).await;

        let headers = resp.headers();
        assert_eq!(headers[CONTENT_TYPE], mime_types::JSON);

        let body = serde_json::from_slice(resp.body()).expect("response body is JSON");
        assert_eq!(
            self.expect_status_code,
            resp.status(),
            "\nresponse: {}",
            pretty(&body)
        );

        body
    }
}

pub fn pretty(val: &Value) -> String {
    serde_json::to_string_pretty(val).unwrap() + "\n"
}

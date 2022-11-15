// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use rand::SeedableRng;
use reqwest::header::AUTHORIZATION;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use warp::http::header::CONTENT_TYPE;
use warp::http::Response;
use warp::hyper::body::Bytes;

use crate::context::{ClientTuple, JsonWebTokenService, PeerStoreTuple};
use crate::CustomEventConfig;
use crate::{context::Context, index, TelemetryServiceConfig};

use aptos_crypto::{x25519, Uniform};
use aptos_rest_client::aptos_api_types::mime_types;

pub async fn new_test_context() -> TestContext {
    let mut rng = ::rand::rngs::StdRng::from_seed([0u8; 32]);
    let server_private_key = x25519::PrivateKey::generate(&mut rng);

    let config = TelemetryServiceConfig {
        address: format!("{}:{}", "127.0.0.1", 80).parse().unwrap(),
        tls_cert_path: None,
        tls_key_path: None,
        trusted_full_node_addresses: HashMap::new(),
        update_interval: 60,
        custom_event_config: CustomEventConfig {
            project_id: String::from("1"),
            dataset_id: String::from("2"),
            table_id: String::from("3"),
        },
        victoria_metrics_endpoints: HashMap::new(),
        humio_url: "".into(),
        pfn_allowlist: HashMap::new(),
        log_env_map: HashMap::new(),
        metrics_exporter_base_url: "".into(),
        peer_identities: HashMap::new(),
    };

    let peers = PeerStoreTuple::default();
    let jwt_service = JsonWebTokenService::from_base64_secret(&base64::encode("jwt_secret_key"));

    TestContext::new(
        config,
        Context::new(
            server_private_key,
            peers,
            ClientTuple::new(None, Some(BTreeMap::new()), None),
            HashSet::new(),
            jwt_service,
            HashMap::new(),
            HashMap::new(),
        ),
    )
}

#[derive(Clone)]
pub struct TestContext {
    pub config: TelemetryServiceConfig,
    expect_status_code: u16,
    pub inner: Context,
    bearer_token: String,
}

impl TestContext {
    pub fn new(config: TelemetryServiceConfig, context: Context) -> Self {
        Self {
            config,
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

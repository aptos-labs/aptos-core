// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::clients::humio;
use crate::GCPBigQueryConfig;
use crate::{context::Context, index, validator_cache::ValidatorSetCache, TelemetryServiceConfig};
use aptos_config::keys::ConfigKey;
use aptos_crypto::{x25519, Uniform};
use aptos_rest_client::aptos_api_types::mime_types;
use rand::SeedableRng;
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
    let cache = ValidatorSetCache::new(aptos_infallible::RwLock::new(HashMap::new()));
    let humio_client = humio::IngestClient::new(
        Url::parse("http://localhost/").unwrap(),
        config.humio_auth_token.clone(),
    );
    TestContext::new(Context::new(config, cache, None, None, humio_client))
}

#[derive(Clone)]
pub struct TestContext {
    pub expect_status_code: u16,
    pub inner: Context,
}

impl TestContext {
    pub fn new(context: Context) -> Self {
        Self {
            expect_status_code: 200,
            inner: context,
        }
    }

    #[allow(dead_code)]
    pub async fn get(&self, path: &str) -> Value {
        self.execute(warp::test::request().method("GET").path(path))
            .await
    }

    pub async fn post(&self, path: &str, body: Value) -> Value {
        self.execute(warp::test::request().method("POST").path(path).json(&body))
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

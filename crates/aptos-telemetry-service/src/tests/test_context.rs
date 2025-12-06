// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    context::{ClientTuple, Context, GroupedMetricsClients, JsonWebTokenService, PeerStoreTuple},
    index, CustomEventConfig, LogIngestConfig, MetricsEndpointsConfig, TelemetryServiceConfig,
};
use aptos_crypto::{x25519, Uniform};
use aptos_infallible::RwLock;
use aptos_rest_client::aptos_api_types::mime_types;
use rand::SeedableRng;
use reqwest::header::AUTHORIZATION;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use warp::{
    http::{header::CONTENT_TYPE, Response},
    hyper::body::Bytes,
};

pub async fn new_test_context() -> TestContext {
    new_test_context_with_auth(None).await
}

pub async fn new_test_context_with_auth(
    on_chain_auth_config: Option<crate::OnChainAuthConfig>,
) -> TestContext {
    let mut rng = ::rand::rngs::StdRng::from_seed([0u8; 32]);
    let server_private_key = x25519::PrivateKey::generate(&mut rng);

    // Convert the single auth config into the new multi-contract format for testing
    let custom_contract_configs = if let Some(auth_config) = on_chain_auth_config {
        vec![crate::CustomContractConfig {
            name: "test_contract".to_string(),
            on_chain_auth: auth_config,
            metrics_sinks: None,
            logs_sink: None,
            events_sink: None,
        }]
    } else {
        Vec::new()
    };

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
        pfn_allowlist: HashMap::new(),
        log_env_map: HashMap::new(),
        peer_identities: HashMap::new(),
        metrics_endpoints_config: MetricsEndpointsConfig::default_for_test(),
        humio_ingest_config: LogIngestConfig::default_for_test(),
        custom_contract_configs,
        allowlist_cache_ttl_secs: 10, // Short TTL for testing
    };

    let peers = PeerStoreTuple::default();
    let jwt_service = JsonWebTokenService::from_base64_secret(&base64::encode("jwt_secret_key"));

    // Build custom contract clients if configured
    let custom_contract_clients = if !config.custom_contract_configs.is_empty() {
        let mut instances = HashMap::new();
        for cc_config in &config.custom_contract_configs {
            instances.insert(
                cc_config.name.clone(),
                crate::context::CustomContractInstance {
                    config: cc_config.on_chain_auth.clone(),
                    metrics_clients: HashMap::new(),
                    logs_client: None,
                    bigquery_client: None,
                },
            );
        }
        Some(crate::context::CustomContractClients { instances })
    } else {
        None
    };

    TestContext::new(
        config.clone(),
        Context::new(
            server_private_key,
            peers,
            ClientTuple::new(
                None,
                Some(GroupedMetricsClients::new_empty()),
                None,
                custom_contract_clients,
            ),
            jwt_service,
            HashMap::new(),
            HashMap::new(),
            Arc::new(RwLock::new(HashMap::new())),
            config.allowlist_cache_ttl_secs,
        ),
    )
}

#[derive(Clone)]
pub struct TestContext {
    #[allow(dead_code)]
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

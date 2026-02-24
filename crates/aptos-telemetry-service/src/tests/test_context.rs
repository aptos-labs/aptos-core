// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    context::{ClientTuple, Context, GroupedMetricsClients, JsonWebTokenService, PeerStoreTuple},
    index,
    rate_limiter::ContractRateLimiters,
    CustomEventConfig, LogIngestConfig, MetricsEndpointsConfig, TelemetryServiceConfig,
    UnknownTelemetryRateLimitConfig,
};
use aptos_crypto::{x25519, Uniform};
use aptos_infallible::RwLock;
use aptos_rest_client::aptos_api_types::mime_types;
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
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
    // Wrap in a vec with default contract name
    let configs = on_chain_auth_config.map(|cfg| vec![("test_contract".to_string(), cfg)]);
    new_test_context_with_multiple_contracts(configs).await
}

/// Create test context with multiple custom contracts (for cross-contract testing)
pub async fn new_test_context_with_multiple_contracts(
    contract_configs: Option<Vec<(String, crate::OnChainAuthConfig)>>,
) -> TestContext {
    let mut rng = ::rand::rngs::StdRng::from_seed([0u8; 32]);
    let server_private_key = x25519::PrivateKey::generate(&mut rng);

    // Convert to multi-contract format
    let custom_contract_configs = contract_configs
        .map(|configs| {
            configs
                .into_iter()
                .map(|(name, auth_config)| crate::CustomContractConfig {
                    name,
                    on_chain_auth: Some(auth_config),
                    static_allowlist: HashMap::new(),
                    node_type_name: Some("test_node_type".to_string()),
                    allow_unknown_nodes: false,
                    metrics_sink: None,
                    metrics_sinks: None,
                    logs_sink: None,
                    events_sink: None,
                    untrusted_metrics_sinks: None,
                    untrusted_logs_sink: None,
                    untrusted_metrics_rate_limit: None,
                    untrusted_logs_rate_limit: None,
                    peer_identities: HashMap::new(),
                    blacklist_peers: None,
                })
                .collect()
        })
        .unwrap_or_default();

    let config = TelemetryServiceConfig {
        address: format!("{}:{}", "127.0.0.1", 80).parse().unwrap(),
        tls_cert_path: None,
        tls_key_path: None,
        trusted_full_node_addresses: HashMap::new(),
        update_interval: 60,
        custom_event_config: Some(CustomEventConfig {
            project_id: String::from("1"),
            dataset_id: String::from("2"),
            table_id: String::from("3"),
        }),
        pfn_allowlist: HashMap::new(),
        log_env_map: HashMap::new(),
        peer_identities: HashMap::new(),
        metrics_endpoints_config: Some(MetricsEndpointsConfig::default_for_test()),
        humio_ingest_config: Some(LogIngestConfig::default_for_test()),
        custom_contract_configs,
        allowlist_cache_ttl_secs: 10, // Short TTL for testing
        unknown_metrics_rate_limit: UnknownTelemetryRateLimitConfig::default(),
        unknown_logs_rate_limit: UnknownTelemetryRateLimitConfig::default(),
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
                    static_allowlist: cc_config.static_allowlist.clone(),
                    node_type_name: cc_config.effective_node_type_name(),
                    allow_unknown_nodes: cc_config.allow_unknown_nodes,
                    metrics_clients: HashMap::new(),
                    untrusted_metrics_clients: HashMap::new(),
                    logs_client: None,
                    untrusted_logs_client: None,
                    bigquery_client: None,
                    peer_identities: cc_config.peer_identities.clone(),
                    blacklist_peers: cc_config.blacklist_peers.clone(),
                },
            );
        }
        Some(crate::context::CustomContractClients { instances })
    } else {
        None
    };

    // Create disabled rate limiters for tests (high burst capacity to avoid interfering with tests)
    let metrics_rate_limit_config = UnknownTelemetryRateLimitConfig {
        requests_per_second: 10000,
        burst_capacity: 100000,
        enabled: false, // Disabled for tests
    };
    let logs_rate_limit_config = UnknownTelemetryRateLimitConfig {
        requests_per_second: 10000,
        burst_capacity: 100000,
        enabled: false, // Disabled for tests
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
            metrics_rate_limit_config,
            logs_rate_limit_config,
            Arc::new(ContractRateLimiters::new()),
            Arc::new(ContractRateLimiters::new()),
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

    /// Pre-populate the allowlist cache with test addresses.
    /// This is needed because tests don't run the AllowlistCacheUpdater background task.
    pub fn populate_allowlist_cache(
        &self,
        contract_name: &str,
        chain_id: ChainId,
        addresses: Vec<AccountAddress>,
    ) {
        self.inner
            .allowlist_cache()
            .update(contract_name, &chain_id, addresses);
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

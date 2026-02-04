// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    allowlist_cache::AllowlistCacheUpdater,
    clients::{big_query, humio, loki, prometheus_remote_write, victoria_metrics},
    context::{
        ClientTuple, Context, JsonWebTokenService, LogIngestClients, MetricsIngestClient,
        PeerStoreTuple,
    },
    index::routes,
    metrics::PrometheusExporter,
    peer_location::PeerLocationUpdater,
    rate_limiter::ContractRateLimiters,
    validator_cache::PeerSetCacheUpdater,
};
use aptos_crypto::{x25519, ValidCryptoMaterialStringExt};
use aptos_types::{chain_id::ChainId, PeerId};
use clap::Parser;
use context::GroupedMetricsClients;
use gcp_bigquery_client::Client as BigQueryClient;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    env,
    fs::File,
    io::Read,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use types::common::ChainCommonName;
use warp::{Filter, Reply};

mod allowlist_cache;
mod auth;
mod challenge_cache;
mod clients;
mod constants;
mod context;
mod custom_contract_auth;
mod custom_contract_ingest;
mod custom_event;
mod errors;
mod gcp_logger;
mod index;
mod jwt_auth;
mod log_ingest;
mod metrics;
mod peer_location;
mod prometheus_push_metrics;
mod rate_limiter;
mod remote_config;
#[cfg(test)]
pub(crate) mod tests;
pub mod types;
mod validator_cache;

#[derive(Clone, Debug, Parser)]
#[clap(name = "Aptos Telemetry Service", author, version)]
pub struct AptosTelemetryServiceArgs {
    #[clap(short = 'f', long, value_parser)]
    config_path: PathBuf,
}

impl AptosTelemetryServiceArgs {
    pub async fn run(self) {
        // Load the config file
        let config =
            TelemetryServiceConfig::load(self.config_path.clone()).unwrap_or_else(|error| {
                panic!(
                    "Failed to load config file: {:?}. Error: {:?}",
                    self.config_path, error
                )
            });
        info!("Using config {:?}", &config);

        let server_private_key = x25519::PrivateKey::from_encoded_string(
            env::var("SERVER_PRIVATE_KEY")
                .expect("environment variable SERVER_PRIVATE_KEY must be set")
                .as_str(),
        )
        .expect("unable to form x25519::Private key from environment variable SERVER_PRIVATE_KEY");

        // BigQuery is optional - skip if GOOGLE_APPLICATION_CREDENTIALS not set or invalid
        let bigquery_client: Option<BigQueryClient> =
            match env::var("GOOGLE_APPLICATION_CREDENTIALS") {
                Ok(creds_path) if !creds_path.is_empty() && creds_path != "/dev/null" => {
                    match BigQueryClient::from_service_account_key_file(&creds_path).await {
                        Ok(client) => {
                            info!("BigQuery client initialized successfully");
                            Some(client)
                        },
                        Err(e) => {
                            warn!(
                                "Failed to create BigQuery client (BigQuery features disabled): {}",
                                e
                            );
                            None
                        },
                    }
                },
                _ => {
                    warn!("GOOGLE_APPLICATION_CREDENTIALS not set - BigQuery features disabled");
                    None
                },
            };

        // BigQuery table client for standard custom events (requires both BigQuery client and custom_event_config)
        let bigquery_table_client = bigquery_client
            .as_ref()
            .zip(config.custom_event_config.as_ref())
            .map(|(client, event_config)| {
                big_query::TableWriteClient::new(
                    client.clone(),
                    event_config.project_id.clone(),
                    event_config.dataset_id.clone(),
                    event_config.table_id.clone(),
                )
            });

        // Standard metrics clients (optional - not needed for custom-contract-only mode)
        let (metrics_clients, telemetry_metrics_client) = if let Some(metrics_config) =
            config.metrics_endpoints_config.clone()
        {
            let clients: GroupedMetricsClients = metrics_config.into();
            let telemetry_client = clients
                .telemetry_service_metrics_clients
                .values()
                .next()
                .cloned();
            (Some(clients), telemetry_client)
        } else {
            info!("Standard metrics endpoints not configured - standard node metrics ingestion disabled");
            (None, None)
        };

        // Standard log ingest clients (optional - not needed for custom-contract-only mode)
        let log_ingest_clients = config.humio_ingest_config.clone().map(|cfg| {
            let clients: LogIngestClients = cfg.into();
            clients
        });
        if log_ingest_clients.is_none() {
            info!("Standard log ingestion not configured - standard node log ingestion disabled");
        }

        // Setup custom contract clients from new config format
        let custom_contract_clients = if !config.custom_contract_configs.is_empty() {
            let mut instances = HashMap::new();

            for cc_config in &config.custom_contract_configs {
                // Validate that the contract name is unique
                if instances.contains_key(&cc_config.name) {
                    panic!(
                        "Duplicate custom contract name '{}' found in configuration. \
                         Each custom contract must have a unique name.",
                        cc_config.name
                    );
                }

                // Validate the configuration (panics if invalid)
                cc_config.validate();

                // Merge metrics clients from both metrics_sink and metrics_sinks
                let metrics_clients = cc_config.make_metrics_clients();
                let untrusted_metrics_clients = cc_config.make_untrusted_metrics_clients();

                let logs_client = cc_config.logs_sink.as_ref().map(|s| s.make_client());
                let untrusted_logs_client = cc_config
                    .untrusted_logs_sink
                    .as_ref()
                    .map(|s| s.make_client());

                let cc_bigquery_client = cc_config.events_sink.as_ref().and_then(|ec| {
                    bigquery_client.as_ref().map(|client| {
                        big_query::TableWriteClient::new(
                            client.clone(),
                            ec.project_id.clone(),
                            ec.dataset_id.clone(),
                            ec.table_id.clone(),
                        )
                    })
                });

                // Log the contract mode
                if cc_config.on_chain_auth.is_none() {
                    info!(
                        "Custom contract '{}' in open telemetry mode (no on_chain_auth, all nodes treated as unknown)",
                        cc_config.name
                    );
                } else if cc_config.allow_unknown_nodes {
                    info!(
                        "Custom contract '{}' allows unknown/untrusted nodes",
                        cc_config.name
                    );
                }

                instances.insert(cc_config.name.clone(), context::CustomContractInstance {
                    config: cc_config.on_chain_auth.clone(),
                    static_allowlist: cc_config.static_allowlist.clone(),
                    node_type_name: cc_config.effective_node_type_name(),
                    allow_unknown_nodes: cc_config.allow_unknown_nodes,
                    metrics_clients,
                    untrusted_metrics_clients,
                    logs_client,
                    untrusted_logs_client,
                    bigquery_client: cc_bigquery_client,
                    peer_identities: cc_config.peer_identities.clone(),
                    blacklist_peers: cc_config.blacklist_peers.clone(),
                });
            }

            Some(context::CustomContractClients { instances })
        } else {
            None
        };

        let jwt_service = JsonWebTokenService::from_base64_secret(
            env::var("JWT_SIGNING_KEY")
                .expect("environment variable JWT_SIGNING_KEY must be set")
                .as_str(),
        );

        let validators = Arc::new(aptos_infallible::RwLock::new(HashMap::new()));
        let validator_fullnodes = Arc::new(aptos_infallible::RwLock::new(HashMap::new()));
        let peer_locations = Arc::new(aptos_infallible::RwLock::new(HashMap::new()));
        let public_fullnodes = config.pfn_allowlist.clone();

        // Build per-contract rate limiters for contracts that have untrusted rate limits configured
        let contract_metrics_rate_limiters = Arc::new(ContractRateLimiters::new());
        let contract_logs_rate_limiters = Arc::new(ContractRateLimiters::new());
        for cc_config in &config.custom_contract_configs {
            // Use blocking to add limiters since we're in async context
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    if let Some(ref rate_limit) = cc_config.untrusted_metrics_rate_limit {
                        info!(
                            "Custom contract '{}' has untrusted metrics rate limit: {} rps, {} burst",
                            cc_config.name, rate_limit.requests_per_second, rate_limit.burst_capacity
                        );
                        contract_metrics_rate_limiters
                            .add_limiter(cc_config.name.clone(), rate_limit.clone())
                            .await;
                    }
                    if let Some(ref rate_limit) = cc_config.untrusted_logs_rate_limit {
                        info!(
                            "Custom contract '{}' has untrusted logs rate limit: {} rps, {} burst",
                            cc_config.name, rate_limit.requests_per_second, rate_limit.burst_capacity
                        );
                        contract_logs_rate_limiters
                            .add_limiter(cc_config.name.clone(), rate_limit.clone())
                            .await;
                    }
                })
            });
        }

        let context = Context::new(
            server_private_key,
            PeerStoreTuple::new(
                validators.clone(),
                validator_fullnodes.clone(),
                public_fullnodes,
            ),
            ClientTuple::new(
                bigquery_table_client,
                metrics_clients,
                log_ingest_clients,
                custom_contract_clients,
            ),
            jwt_service,
            config.log_env_map.clone(),
            config.peer_identities.clone(),
            peer_locations.clone(),
            config.unknown_metrics_rate_limit.clone(),
            config.unknown_logs_rate_limit.clone(),
            contract_metrics_rate_limiters,
            contract_logs_rate_limiters,
        );

        // Only start PeerSetCacheUpdater if there are chains to update
        if !config.trusted_full_node_addresses.is_empty() {
            PeerSetCacheUpdater::new(
                validators,
                validator_fullnodes,
                config.trusted_full_node_addresses.clone(),
                Duration::from_secs(config.update_interval),
            )
            .run();
        } else {
            info!("No trusted_full_node_addresses configured - validator cache updates disabled");
        }

        // PeerLocationUpdater requires BigQuery - only start if available
        if let Some(bq_client) = bigquery_client.as_ref() {
            if let Err(err) =
                PeerLocationUpdater::new(bq_client.clone(), peer_locations.clone()).run()
            {
                error!("Failed to start PeerLocationUpdater: {:?}", err);
            }
        } else {
            info!("PeerLocationUpdater disabled - BigQuery client not available");
        }

        // PrometheusExporter exports service metrics - only start if telemetry metrics endpoint configured
        if let Some(telemetry_client) = telemetry_metrics_client {
            PrometheusExporter::new(telemetry_client).run();
        } else {
            info!("PrometheusExporter disabled - no telemetry service metrics endpoint configured");
        }

        // Start AllowlistCacheUpdater for custom contracts (like PeerSetCacheUpdater)
        // This keeps the allowlist cache fresh in the background
        if !config.custom_contract_configs.is_empty() {
            info!(
                "Starting AllowlistCacheUpdater for {} custom contracts (interval: {}s)",
                config.custom_contract_configs.len(),
                config.allowlist_cache_ttl_secs
            );
            AllowlistCacheUpdater::new(
                context.allowlist_cache().clone(),
                config.custom_contract_configs.clone(),
                Duration::from_secs(config.allowlist_cache_ttl_secs),
            )
            .run();
        }

        Self::serve(&config, routes(context)).await;
    }

    async fn serve<F>(config: &TelemetryServiceConfig, routes: F)
    where
        F: Filter<Error = Infallible> + Clone + Sync + Send + 'static,
        F::Extract: Reply,
    {
        match &config.tls_cert_path {
            None => warp::serve(routes).bind(config.address).await,
            Some(cert_path) => {
                warp::serve(routes)
                    .tls()
                    .cert_path(cert_path)
                    .key_path(config.tls_key_path.as_ref().unwrap())
                    .bind(config.address)
                    .await
            },
        };
    }
}

/// Per-metric endpoint configuration.
///
/// Supports multiple named endpoints (e.g., for failover or multi-region) and
/// configurable backend type. Authentication can be bearer tokens OR basic auth.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsEndpoint {
    /// Map of endpoint name to URL (e.g., {"default": "http://vm:8428/..."})
    endpoint_urls: HashMap<String, Url>,

    /// Authentication type (bearer, basic, or none). Defaults to bearer.
    #[serde(default)]
    auth_type: SinkAuthType,

    /// Environment variable containing JSON map of endpoint names to bearer tokens.
    /// Used when auth_type is "bearer" (default).
    keys_env_var: String,

    /// Environment variable containing JSON map of endpoint names to basic auth creds.
    /// Format: {"endpoint_name": "username:password", ...}
    /// Used when auth_type is "basic".
    #[serde(default)]
    basic_auth_env_var: Option<String>,

    /// Backend type (victoria_metrics or prometheus_remote_write). Defaults to victoria_metrics.
    #[serde(default)]
    backend_type: MetricsBackendType,
}

impl MetricsEndpoint {
    #[cfg(test)]
    fn default_for_test() -> Self {
        Self {
            endpoint_urls: HashMap::new(),
            auth_type: SinkAuthType::Bearer,
            keys_env_var: "".into(),
            basic_auth_env_var: None,
            backend_type: MetricsBackendType::VictoriaMetrics,
        }
    }

    fn make_client(&self) -> HashMap<String, MetricsIngestClient> {
        self.endpoint_urls
            .iter()
            .map(|(name, url)| {
                let auth_token = self.get_auth_token(name);
                let client = match self.backend_type {
                    MetricsBackendType::VictoriaMetrics => MetricsIngestClient::VictoriaMetrics(
                        victoria_metrics::VictoriaMetricsClient::new(url.clone(), auth_token),
                    ),
                    MetricsBackendType::PrometheusRemoteWrite => {
                        MetricsIngestClient::PrometheusRemoteWrite(
                            prometheus_remote_write::PrometheusRemoteWriteClient::new(
                                url.clone(),
                                auth_token,
                            ),
                        )
                    },
                };
                (name.clone(), client)
            })
            .collect()
    }

    /// Get auth token for a specific endpoint based on auth_type
    fn get_auth_token(&self, endpoint_name: &str) -> victoria_metrics::AuthToken {
        match self.auth_type {
            SinkAuthType::Bearer => {
                let secrets: HashMap<String, String> =
                    serde_json::from_str(&env::var(&self.keys_env_var).unwrap_or_else(|_| {
                        panic!(
                            "environment variable {} must be set for bearer auth",
                            self.keys_env_var
                        )
                    }))
                    .unwrap_or_else(|_| {
                        panic!(
                            "environment variable {} must be a JSON map of name to secret",
                            self.keys_env_var
                        )
                    });
                let secret = secrets.get(endpoint_name).unwrap_or_else(|| {
                    panic!(
                        "environment variable {} is missing secret for {}",
                        self.keys_env_var, endpoint_name
                    )
                });
                victoria_metrics::AuthToken::Bearer(secret.clone())
            },
            SinkAuthType::Basic => {
                let creds: HashMap<String, String> = self
                    .basic_auth_env_var
                    .as_ref()
                    .and_then(|env_var| env::var(env_var).ok())
                    .and_then(|json_str| serde_json::from_str(&json_str).ok())
                    .expect("basic_auth_env_var must be set and contain valid JSON for basic auth");
                let cred = creds.get(endpoint_name).unwrap_or_else(|| {
                    panic!(
                        "basic_auth_env_var is missing credentials for {}",
                        endpoint_name
                    )
                });
                let parts: Vec<&str> = cred.splitn(2, ':').collect();
                if parts.len() == 2 {
                    victoria_metrics::AuthToken::Basic(parts[0].to_string(), parts[1].to_string())
                } else {
                    panic!(
                        "basic auth for {} must be in 'username:password' format",
                        endpoint_name
                    )
                }
            },
            SinkAuthType::None => victoria_metrics::AuthToken::None,
        }
    }
}

/// Metrics endpoints configuration for different data sources (node telemetry only).
///
/// Supports multiple backends (victoria_metrics, prometheus) via `backend_type` field in each endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsEndpointsConfig {
    /// Endpoint for telemetry service's own metrics (self-monitoring)
    pub telemetry_service_metrics: MetricsEndpoint,

    /// Endpoint for metrics from known/trusted nodes (validators, whitelisted)
    pub ingest_metrics: MetricsEndpoint,

    /// Endpoint for metrics from unknown/untrusted nodes
    pub untrusted_ingest_metrics: MetricsEndpoint,
}

impl MetricsEndpointsConfig {
    #[cfg(test)]
    fn default_for_test() -> Self {
        Self {
            telemetry_service_metrics: MetricsEndpoint::default_for_test(),
            ingest_metrics: MetricsEndpoint::default_for_test(),
            untrusted_ingest_metrics: MetricsEndpoint::default_for_test(),
        }
    }
}

/// Log backend type
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogBackendType {
    #[default]
    Humio,
    Loki,
}

/// Metrics backend type
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricsBackendType {
    /// VictoriaMetrics - uses text format via /api/v1/import/prometheus (default)
    #[default]
    VictoriaMetrics,
    /// Prometheus Remote Write - uses protobuf+snappy via /api/v1/write
    PrometheusRemoteWrite,
}

/// Authentication type for sink endpoints
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SinkAuthType {
    /// Bearer token authentication (default)
    #[default]
    Bearer,
    /// Basic authentication (username:password)
    Basic,
    /// No authentication
    None,
}

/// Authentication method for on-chain verification
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OnChainAuthMethod {
    /// Use get_account_resource API (legacy)
    Resource,
    /// Use view function API (recommended)
    #[default]
    ViewFunction,
}

/// Default node type name for custom contract auth
fn default_node_type_name() -> String {
    "custom".to_string()
}

/// Configuration for on-chain contract-based authentication
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OnChainAuthConfig {
    /// Chain ID where the contract is deployed.
    /// Used for cache keying and pre-warming at startup.
    /// If not specified, defaults to 1 (mainnet).
    #[serde(default = "default_chain_id")]
    pub chain_id: u8,

    /// Authentication method (resource or view_function)
    #[serde(default)]
    pub method: OnChainAuthMethod,

    /// For ViewFunction method: full function path (e.g., "0x123::module::get_members")
    /// For Resource method: full resource path (e.g., "0x123::module::ResourceName")
    /// Can use environment variable substitution with ${ENV_VAR} syntax
    pub resource_path: String,

    /// Arguments to pass to the view function (only used with ViewFunction method)
    /// Each argument is a string that will be passed as-is to the view function
    /// Can use environment variable substitution with ${ENV_VAR} syntax
    /// Example: ["0x1234...", "100"]
    #[serde(default)]
    pub view_function_args: Vec<String>,

    /// JSON path to the list of addresses in the response/resource
    /// Examples: "providers", "members", "data.allowlist", "[0].address"
    pub address_list_field: String,

    /// Optional: chain-specific REST API URL
    /// If not provided, uses default URLs or APTOS_REST_URL_CHAIN_<id> env var
    #[serde(default)]
    pub rest_api_url: Option<Url>,

    /// Custom node type name for authenticated clients
    /// Examples: "ShelbyStorageProvider", "CustomStorageNode", "DataProvider"
    /// Defaults to "custom" if not specified
    #[serde(default = "default_node_type_name")]
    pub node_type_name: String,
}

/// Default chain ID (mainnet)
fn default_chain_id() -> u8 {
    1
}

/// Metrics sink configuration for custom contracts.
///
/// Supports multiple endpoints with a shared backend type.
/// Authentication can be configured via bearer tokens OR basic auth.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsSinkConfig {
    /// Map of sink name to endpoint URL
    #[serde(alias = "endpoints")]
    pub endpoint_urls: HashMap<String, String>,

    /// Authentication type (bearer, basic, or none). Defaults to bearer.
    #[serde(default)]
    pub auth_type: SinkAuthType,

    /// Environment variable containing JSON map of sink names to bearer tokens.
    /// Used when auth_type is "bearer" (default).
    #[serde(default, alias = "keys_env")]
    pub keys_env_var: Option<String>,

    /// Environment variable containing JSON map of sink names to basic auth credentials.
    /// Format: {"sink_name": "username:password", ...}
    /// Used when auth_type is "basic".
    #[serde(default)]
    pub basic_auth_env_var: Option<String>,

    /// Backend type (victoria_metrics or prometheus_remote_write). Defaults to victoria_metrics.
    #[serde(default)]
    pub backend_type: MetricsBackendType,
}

impl MetricsSinkConfig {
    /// Convert to MetricsIngestClient instances
    pub fn make_clients(&self) -> HashMap<String, MetricsIngestClient> {
        self.endpoint_urls
            .iter()
            .map(|(name, url)| {
                let auth_token = self.get_auth_token(name);
                let parsed_url = Url::parse(url).expect("valid URL in metrics sink config");
                let client = match self.backend_type {
                    MetricsBackendType::VictoriaMetrics => MetricsIngestClient::VictoriaMetrics(
                        victoria_metrics::VictoriaMetricsClient::new(parsed_url, auth_token),
                    ),
                    MetricsBackendType::PrometheusRemoteWrite => {
                        MetricsIngestClient::PrometheusRemoteWrite(
                            prometheus_remote_write::PrometheusRemoteWriteClient::new(
                                parsed_url, auth_token,
                            ),
                        )
                    },
                };
                (name.clone(), client)
            })
            .collect()
    }

    /// Get auth token for a specific sink based on auth_type
    fn get_auth_token(&self, sink_name: &str) -> victoria_metrics::AuthToken {
        match self.auth_type {
            SinkAuthType::Bearer => {
                let keys: HashMap<String, String> = self
                    .keys_env_var
                    .as_ref()
                    .and_then(|env_var| std::env::var(env_var).ok())
                    .and_then(|json_str| serde_json::from_str(&json_str).ok())
                    .unwrap_or_default();
                keys.get(sink_name)
                    .map(|k| victoria_metrics::AuthToken::Bearer(k.clone()))
                    // Fall back to None to avoid sending malformed auth headers
                    .unwrap_or(victoria_metrics::AuthToken::None)
            },
            SinkAuthType::Basic => {
                let creds: HashMap<String, String> = self
                    .basic_auth_env_var
                    .as_ref()
                    .and_then(|env_var| std::env::var(env_var).ok())
                    .and_then(|json_str| serde_json::from_str(&json_str).ok())
                    .unwrap_or_default();
                creds
                    .get(sink_name)
                    .and_then(|cred| {
                        let parts: Vec<&str> = cred.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            Some(victoria_metrics::AuthToken::Basic(
                                parts[0].to_string(),
                                parts[1].to_string(),
                            ))
                        } else {
                            None
                        }
                    })
                    // Fall back to None to avoid sending malformed auth headers
                    .unwrap_or(victoria_metrics::AuthToken::None)
            },
            SinkAuthType::None => victoria_metrics::AuthToken::None,
        }
    }
}

/// Log sink configuration for custom contracts.
///
/// Mirrors `MetricsSinkConfig` for consistency. Supports single endpoint.
/// Authentication can be configured via bearer tokens OR basic auth.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogSinkConfig {
    /// Endpoint URL for log ingestion
    #[serde(alias = "endpoint")]
    pub endpoint_url: String,

    /// Authentication type (bearer, basic, or none). Defaults to bearer.
    #[serde(default)]
    pub auth_type: SinkAuthType,

    /// Environment variable containing the bearer token.
    /// Used when auth_type is "bearer" (default).
    #[serde(default, alias = "key_env")]
    pub key_env_var: Option<String>,

    /// Environment variable containing basic auth credentials.
    /// Format: "username:password"
    /// Used when auth_type is "basic".
    #[serde(default)]
    pub basic_auth_env_var: Option<String>,

    /// Backend type (humio or loki). Defaults to humio.
    #[serde(default)]
    pub backend_type: LogBackendType,
}

impl LogSinkConfig {
    /// Convert to LogIngestClient
    pub fn make_client(&self) -> context::LogIngestClient {
        use crate::clients::{humio, loki};

        let parsed_url = Url::parse(&self.endpoint_url).expect("valid URL in log sink config");

        match self.backend_type {
            LogBackendType::Humio => {
                let auth = self.get_humio_auth();
                context::LogIngestClient::Humio(humio::IngestClient::with_auth(parsed_url, auth))
            },
            LogBackendType::Loki => {
                let auth = self.get_loki_auth();
                context::LogIngestClient::Loki(loki::LokiIngestClient::with_auth(parsed_url, auth))
            },
        }
    }

    /// Get Humio auth configuration based on auth_type
    fn get_humio_auth(&self) -> humio::HumioAuth {
        use crate::clients::humio;

        match self.auth_type {
            SinkAuthType::Bearer | SinkAuthType::None => {
                let token = self
                    .key_env_var
                    .as_ref()
                    .and_then(|env_var| env::var(env_var).ok())
                    .unwrap_or_else(|| {
                        if matches!(self.auth_type, SinkAuthType::Bearer) {
                            panic!("key_env_var must be set for Humio with bearer auth")
                        }
                        "".to_string()
                    });
                humio::HumioAuth::Bearer(token)
            },
            SinkAuthType::Basic => {
                let creds = self
                    .basic_auth_env_var
                    .as_ref()
                    .and_then(|env_var| env::var(env_var).ok())
                    .expect("basic_auth_env_var must be set for basic auth");
                humio::HumioAuth::from_basic_auth_string(&creds)
                    .expect("basic_auth_env_var must be in 'username:password' format")
            },
        }
    }

    /// Get Loki auth configuration based on auth_type
    fn get_loki_auth(&self) -> loki::LokiAuth {
        use crate::clients::loki;

        match self.auth_type {
            SinkAuthType::None => loki::LokiAuth::None,
            SinkAuthType::Bearer => {
                let token = self
                    .key_env_var
                    .as_ref()
                    .and_then(|env_var| env::var(env_var).ok());
                loki::LokiAuth::from_bearer_token(token)
            },
            SinkAuthType::Basic => {
                let creds = self
                    .basic_auth_env_var
                    .as_ref()
                    .and_then(|env_var| env::var(env_var).ok())
                    .expect("basic_auth_env_var must be set for basic auth");
                loki::LokiAuth::from_basic_auth_string(&creds)
                    .expect("basic_auth_env_var must be in 'username:password' format")
            },
        }
    }
}

/// Custom contract configuration - consolidates auth and all data sinks.
///
/// All sink configs follow a consistent pattern:
/// - `metrics_sink`: Single MetricsSinkConfig (backwards compatible)
/// - `metrics_sinks`: Array of MetricsSinkConfig (for multiple backend types)
/// - `logs_sink`: Uses `LogSinkConfig` (single-endpoint, backend_type)
/// - `events_sink`: Uses `CustomEventConfig` (BigQuery)
///
/// For multiple metrics sinks with different backend types (e.g., victoria_metrics AND
/// prometheus_remote_write), use the `metrics_sinks` array field. Both `metrics_sink`
/// and `metrics_sinks` can be used together - they will be merged.
///
/// ## Open Telemetry Mode (no on_chain_auth)
///
/// When `on_chain_auth` is omitted, `allow_unknown_nodes` MUST be `true`.
/// In this mode, ALL nodes are treated as unknown/untrusted. Signature verification
/// is still required (nodes must prove address ownership), but no on-chain allowlist
/// check is performed. This is useful for collecting telemetry from community nodes
/// without requiring an on-chain registry.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomContractConfig {
    /// Unique identifier for this custom contract configuration.
    /// Used in routing and logging.
    pub name: String,

    /// On-chain authentication configuration (optional).
    /// When provided, nodes are verified against an on-chain allowlist.
    /// When omitted, `allow_unknown_nodes` MUST be true and all nodes are treated as unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_chain_auth: Option<OnChainAuthConfig>,

    /// Static allowlist - addresses here are treated as "trusted" without on-chain verification.
    /// Maps chain_id -> set of addresses. Useful for RPCs where you know the operators
    /// but don't want on-chain overhead.
    ///
    /// Trust determination priority:
    /// 1. On-chain allowlist (if `on_chain_auth` configured) → trusted
    /// 2. Static allowlist → trusted
    /// 3. `allow_unknown_nodes: true` → untrusted
    /// 4. Otherwise → rejected
    ///
    /// This allows you to grant "trusted" status to known addresses via config,
    /// routing their telemetry to trusted sinks without on-chain verification.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub static_allowlist: HashMap<ChainId, HashSet<PeerId>>,

    /// Custom node type name for labeling telemetry from this contract.
    /// Used in metrics labels as `node_type={node_type_name}`.
    /// If not specified, falls back to `on_chain_auth.node_type_name` (if on_chain_auth is configured),
    /// otherwise defaults to "custom".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_type_name: Option<String>,

    /// Allow unknown/untrusted nodes to authenticate via this custom contract endpoint.
    /// When true, nodes that are NOT in the on-chain allowlist can still get a JWT token
    /// (with NodeType::CustomUnknown) and send telemetry. Their data is routed to
    /// `untrusted_metrics_sinks` and `untrusted_logs_sink` instead of the trusted sinks.
    /// This enables custom labeling/attribution for community nodes.
    /// Default: false (only allowlisted nodes can authenticate)
    #[serde(default)]
    pub allow_unknown_nodes: bool,

    /// Single metrics sink for this custom contract (optional, backwards compatible).
    /// Use `metrics_sinks` (array) if you need multiple sinks with different backend types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_sink: Option<MetricsSinkConfig>,

    /// Multiple metrics sinks for this custom contract (optional).
    /// Use this when you need different backend types (e.g., victoria_metrics AND prometheus_remote_write).
    /// Can be used alongside `metrics_sink` - all sinks will be merged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_sinks: Option<Vec<MetricsSinkConfig>>,

    /// Log sink for this custom contract (optional).
    /// Supports single endpoint with selectable backend (humio or loki).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs_sink: Option<LogSinkConfig>,

    /// BigQuery events sink for this custom contract (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events_sink: Option<CustomEventConfig>,

    // ========================================================================
    // Untrusted/Unknown Node Sinks (only used when allow_unknown_nodes: true)
    // ========================================================================
    /// Metrics sinks for unknown/untrusted nodes (optional).
    /// Used when `allow_unknown_nodes: true` and the authenticating node is NOT in the allowlist.
    /// If not specified, unknown nodes will use the regular `metrics_sinks`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub untrusted_metrics_sinks: Option<Vec<MetricsSinkConfig>>,

    /// Log sink for unknown/untrusted nodes (optional).
    /// Used when `allow_unknown_nodes: true` and the authenticating node is NOT in the allowlist.
    /// If not specified, unknown nodes will use the regular `logs_sink`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub untrusted_logs_sink: Option<LogSinkConfig>,

    /// Rate limit configuration for untrusted nodes' metrics ingestion (optional).
    /// When specified, overrides the global `unknown_metrics_rate_limit` for this contract.
    /// Only applies to nodes that fail allowlist verification (is_trusted: false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub untrusted_metrics_rate_limit: Option<UnknownTelemetryRateLimitConfig>,

    /// Rate limit configuration for untrusted nodes' logs ingestion (optional).
    /// When specified, overrides the global `unknown_logs_rate_limit` for this contract.
    /// Only applies to nodes that fail allowlist verification (is_trusted: false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub untrusted_logs_rate_limit: Option<UnknownTelemetryRateLimitConfig>,

    // ========================================================================
    // Per-Peer Configuration (optional, for fine-grained control)
    // ========================================================================
    /// Per-peer identity mapping for metrics labeling (optional).
    /// Maps chain_id -> peer_id -> identity/common name.
    /// Used to add `kubernetes_pod_name=peer_id:{identity}//{peer_id}` labels to metrics,
    /// matching the standard telemetry labeling behavior.
    /// If not specified, metrics will use `kubernetes_pod_name=peer_id:{peer_id}`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub peer_identities: HashMap<ChainId, HashMap<PeerId, String>>,

    /// Optional set of peer IDs to block from this contract's telemetry ingestion.
    /// Blocked peers will receive a 403 Forbidden response.
    /// Applies to metrics, logs, and events endpoints for this contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blacklist_peers: Option<HashSet<PeerId>>,
}

impl CustomContractConfig {
    /// Validate the configuration.
    /// Panics if the configuration is invalid.
    pub fn validate(&self) {
        // If neither on_chain_auth nor static_allowlist is configured, allow_unknown_nodes MUST be true
        let has_on_chain = self.on_chain_auth.is_some();
        let has_static = !self.static_allowlist.is_empty();
        if !has_on_chain && !has_static && !self.allow_unknown_nodes {
            panic!(
                "Custom contract '{}' has no on_chain_auth or static_allowlist configured but \
                 allow_unknown_nodes is false. When both allowlists are omitted, \
                 allow_unknown_nodes MUST be true (open telemetry mode).",
                self.name
            );
        }
    }

    /// Check if an address is in the static allowlist for a given chain.
    pub fn is_in_static_allowlist(&self, chain_id: &ChainId, address: &PeerId) -> bool {
        self.static_allowlist
            .get(chain_id)
            .map(|addrs| addrs.contains(address))
            .unwrap_or(false)
    }

    /// Get the effective node type name for this contract.
    /// Priority: 1) explicit node_type_name, 2) on_chain_auth.node_type_name, 3) "custom"
    pub fn effective_node_type_name(&self) -> String {
        self.node_type_name
            .clone()
            .or_else(|| {
                self.on_chain_auth
                    .as_ref()
                    .map(|auth| auth.node_type_name.clone())
            })
            .unwrap_or_else(|| "custom".to_string())
    }

    /// Get all metrics clients from both `metrics_sink` and `metrics_sinks` fields.
    /// This allows backwards compatibility with single sink configs while supporting
    /// multiple sinks with different backend types.
    pub fn make_metrics_clients(&self) -> HashMap<String, MetricsIngestClient> {
        let mut clients = HashMap::new();

        // Add clients from singular metrics_sink (backwards compatible)
        if let Some(ref sink) = self.metrics_sink {
            clients.extend(sink.make_clients());
        }

        // Add clients from plural metrics_sinks array
        if let Some(ref sinks) = self.metrics_sinks {
            for sink in sinks {
                clients.extend(sink.make_clients());
            }
        }

        clients
    }

    /// Get metrics clients for untrusted/unknown nodes.
    /// Falls back to regular metrics clients if no untrusted sinks are configured.
    pub fn make_untrusted_metrics_clients(&self) -> HashMap<String, MetricsIngestClient> {
        // If untrusted sinks are configured, use them
        if let Some(ref sinks) = self.untrusted_metrics_sinks {
            let mut clients = HashMap::new();
            for sink in sinks {
                clients.extend(sink.make_clients());
            }
            if !clients.is_empty() {
                return clients;
            }
        }

        // Fall back to regular metrics clients
        self.make_metrics_clients()
    }
}

impl OnChainAuthConfig {
    /// Resolve environment variables in a string (${ENV_VAR} syntax)
    ///
    /// Note: Only substitutes patterns from the original string. If an env var's
    /// value contains ${VAR} patterns, they are NOT recursively substituted.
    /// This prevents infinite loops from self-referential (FOO=abc${FOO}def)
    /// or cyclical (FOO=${BAR}, BAR=${FOO}) environment variables.
    fn resolve_env_vars(input: &str) -> Result<String, String> {
        let mut resolved = input.to_string();
        let mut search_start = 0;

        // Find and replace ${ENV_VAR} patterns, advancing past each substitution
        while let Some(rel_start) = resolved[search_start..].find("${") {
            let start = search_start + rel_start;
            if let Some(rel_end) = resolved[start..].find('}') {
                let end = start + rel_end;
                let env_var = &resolved[start + 2..end];
                let value = std::env::var(env_var)
                    .map_err(|_| format!("Environment variable {} not set", env_var))?;
                let value_len = value.len();
                resolved = format!("{}{}{}", &resolved[..start], value, &resolved[end + 1..]);
                // Continue searching after the substituted value to avoid infinite loops
                search_start = start + value_len;
            } else {
                return Err("Malformed environment variable substitution".to_string());
            }
        }

        Ok(resolved)
    }

    /// Resolve environment variables in the resource path
    pub fn resolve_resource_path(&self) -> Result<String, String> {
        Self::resolve_env_vars(&self.resource_path)
    }

    /// Resolve environment variables in view function arguments
    pub fn resolve_view_function_args(&self) -> Result<Vec<String>, String> {
        self.view_function_args
            .iter()
            .map(|arg| Self::resolve_env_vars(arg))
            .collect()
    }
}

/// Log ingestion endpoint configuration for main telemetry service.
///
/// Supports bearer tokens OR basic auth, configurable via `auth_type`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogIngestEndpoint {
    /// Endpoint URL for log ingestion
    pub endpoint_url: Url,

    /// Authentication type (bearer, basic, or none). Defaults to bearer.
    #[serde(default)]
    pub auth_type: SinkAuthType,

    /// Environment variable containing the bearer token.
    /// Used when auth_type is "bearer" (default).
    /// Note: Optional for Loki backend (can operate without auth)
    pub key_env_var: String,

    /// Environment variable containing basic auth credentials.
    /// Format: "username:password"
    /// Used when auth_type is "basic".
    #[serde(default)]
    pub basic_auth_env_var: Option<String>,

    /// Backend type (humio or loki). Defaults to humio for backward compatibility.
    #[serde(default)]
    pub backend_type: LogBackendType,
}

impl LogIngestEndpoint {
    #[cfg(test)]
    fn default_for_test() -> Self {
        Self {
            endpoint_url: Url::parse("test://test").unwrap(),
            auth_type: SinkAuthType::Bearer,
            key_env_var: "".into(),
            basic_auth_env_var: None,
            backend_type: LogBackendType::Humio,
        }
    }

    fn make_client(&self) -> context::LogIngestClient {
        match self.backend_type {
            LogBackendType::Humio => {
                let auth = self.get_humio_auth();
                context::LogIngestClient::Humio(humio::IngestClient::with_auth(
                    self.endpoint_url.clone(),
                    auth,
                ))
            },
            LogBackendType::Loki => {
                let auth = self.get_loki_auth();
                context::LogIngestClient::Loki(loki::LokiIngestClient::with_auth(
                    self.endpoint_url.clone(),
                    auth,
                ))
            },
        }
    }

    /// Get Humio auth configuration based on auth_type
    fn get_humio_auth(&self) -> humio::HumioAuth {
        match self.auth_type {
            SinkAuthType::Bearer | SinkAuthType::None => {
                let token = env::var(&self.key_env_var).unwrap_or_else(|_| {
                    if matches!(self.auth_type, SinkAuthType::Bearer)
                        && !self.key_env_var.is_empty()
                    {
                        panic!(
                            "environment variable {} must be set for Humio with bearer auth",
                            self.key_env_var
                        )
                    }
                    "".to_string()
                });
                humio::HumioAuth::Bearer(token)
            },
            SinkAuthType::Basic => {
                let creds = self
                    .basic_auth_env_var
                    .as_ref()
                    .and_then(|env_var| env::var(env_var).ok())
                    .expect("basic_auth_env_var must be set for basic auth");
                humio::HumioAuth::from_basic_auth_string(&creds)
                    .expect("basic_auth_env_var must be in 'username:password' format")
            },
        }
    }

    /// Get Loki auth configuration based on auth_type
    fn get_loki_auth(&self) -> loki::LokiAuth {
        match self.auth_type {
            SinkAuthType::None => loki::LokiAuth::None,
            SinkAuthType::Bearer => {
                let token = env::var(&self.key_env_var).ok();
                loki::LokiAuth::from_bearer_token(token)
            },
            SinkAuthType::Basic => {
                let creds = self
                    .basic_auth_env_var
                    .as_ref()
                    .and_then(|env_var| env::var(env_var).ok())
                    .expect("basic_auth_env_var must be set for basic auth");
                loki::LokiAuth::from_basic_auth_string(&creds)
                    .expect("basic_auth_env_var must be in 'username:password' format")
            },
        }
    }
}

/// Log ingest configuration for different sources.
///
/// Supports multiple backends (humio, loki) via `backend_type` field in each endpoint.
/// This enables gradual migration between log backends while maintaining backward compatibility.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogIngestConfig {
    /// Log endpoint for known/trusted nodes (validators, whitelisted nodes, etc.)
    pub known_logs_endpoint: LogIngestEndpoint,

    /// Log endpoint for unknown/untrusted nodes
    pub unknown_logs_endpoint: LogIngestEndpoint,

    /// Optional set of peer IDs to blacklist from log ingestion
    #[serde(default)]
    pub blacklist_peers: Option<HashSet<PeerId>>,
}
impl LogIngestConfig {
    #[cfg(test)]
    pub(crate) fn default_for_test() -> LogIngestConfig {
        Self {
            known_logs_endpoint: LogIngestEndpoint::default_for_test(),
            unknown_logs_endpoint: LogIngestEndpoint::default_for_test(),
            blacklist_peers: None,
        }
    }
}

/// Main telemetry service configuration.
///
/// Configuration fields use consistent naming patterns:
/// - `*_config` suffix for nested config structs
/// - `backend_type` field to select between backends (e.g., victoria_metrics/prometheus, humio/loki)
/// - Aliases are provided for backward compatibility when field names have been improved
///
/// ## Custom-Contract-Only Mode
///
/// For deployments that only need custom contract telemetry (no standard node auth),
/// set `custom_contract_configs` and leave the standard auth fields at their defaults:
/// - `trusted_full_node_addresses`: empty (no validator sets to fetch)
/// - `pfn_allowlist`: empty (no public fullnode allowlist)
/// - `metrics_endpoints_config`: None (no standard metrics ingestion)
/// - `humio_ingest_config`: None (no standard log ingestion)
/// - `custom_event_config`: None (optional, for BigQuery events)
///
/// Example minimal config:
/// ```yaml
/// address: "0.0.0.0:8080"
/// custom_contract_configs:
///   - name: "my_provider"
///     allow_unknown_nodes: true
///     node_type_name: "MyProvider"
///     metrics_sinks:
///       - endpoint_urls: {vm: "http://metrics:8428/api/v1/import/prometheus"}
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryServiceConfig {
    /// Socket address to bind the service to (e.g., "0.0.0.0:8080")
    pub address: SocketAddr,

    /// Optional TLS certificate path for HTTPS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_cert_path: Option<String>,

    /// Optional TLS key path for HTTPS (required if tls_cert_path is set)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_key_path: Option<String>,

    /// Map of chain name to full node REST API URL for fetching validator sets.
    /// Required for standard node authentication. Can be empty for custom-contract-only mode.
    #[serde(default)]
    pub trusted_full_node_addresses: HashMap<ChainCommonName, String>,

    /// Interval in seconds to update peer/validator caches.
    /// Default: 60 seconds.
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,

    /// Public full node allowlist: chain_id -> peer_id -> public_key.
    /// Required for standard node authentication. Can be empty for custom-contract-only mode.
    #[serde(default)]
    pub pfn_allowlist: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,

    /// BigQuery configuration for custom events (optional).
    /// Required for `/ingest/custom-event` endpoint. Can be omitted for custom-contract-only mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_event_config: Option<CustomEventConfig>,

    /// Log ingestion configuration for known/unknown nodes (optional).
    /// Supports multiple backends (humio, loki) via `backend_type` field in endpoints.
    /// Required for standard `/ingest/logs` endpoint. Can be omitted for custom-contract-only mode.
    /// Note: Field name preserved as `humio_ingest_config` for backward compatibility;
    /// use alias `log_ingest_config` in new configurations.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "log_ingest_config"
    )]
    pub humio_ingest_config: Option<LogIngestConfig>,

    /// Map of chain_id -> peer_id -> environment name (for log routing).
    /// Can be empty for custom-contract-only mode.
    #[serde(default)]
    pub log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,

    /// Map of chain_id -> peer_id -> identity string (for peer identification).
    /// Can be empty for custom-contract-only mode.
    #[serde(default)]
    pub peer_identities: HashMap<ChainId, HashMap<PeerId, String>>,

    /// Metrics endpoints configuration for telemetry service, trusted, and untrusted nodes (optional).
    /// Supports multiple backends (victoria_metrics, prometheus) via `backend_type` field.
    /// Required for standard `/ingest/metrics` endpoint. Can be omitted for custom-contract-only mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_endpoints_config: Option<MetricsEndpointsConfig>,

    /// Custom contract configurations (optional).
    /// Each entry defines authentication and data sinks for a different custom contract client type.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_contract_configs: Vec<CustomContractConfig>,

    /// Allowlist cache TTL in seconds (optional).
    /// Controls how long on-chain allowlist data is cached before re-fetching.
    /// Default: 300 seconds (5 minutes). Set lower for testing (e.g., 10 seconds).
    #[serde(default = "default_allowlist_cache_ttl_secs")]
    pub allowlist_cache_ttl_secs: u64,

    /// Rate limit configuration for unknown/untrusted node metrics ingestion.
    /// Controls rate limiting for standard unknown nodes (UnknownValidator, UnknownFullNode).
    /// Default: 100 requests/second with 200 burst capacity.
    #[serde(default)]
    pub unknown_metrics_rate_limit: UnknownTelemetryRateLimitConfig,

    /// Rate limit configuration for unknown/untrusted node logs ingestion.
    /// Controls rate limiting for standard unknown nodes (Unknown, UnknownValidator, UnknownFullNode).
    /// Default: 100 requests/second with 200 burst capacity.
    #[serde(default)]
    pub unknown_logs_rate_limit: UnknownTelemetryRateLimitConfig,
}

fn default_update_interval() -> u64 {
    60 // 60 seconds default
}

fn default_allowlist_cache_ttl_secs() -> u64 {
    300 // 5 minutes default
}

/// Rate limit configuration for unknown/untrusted telemetry.
///
/// Controls how many requests per time window are allowed from unknown/untrusted nodes.
/// Uses a token bucket algorithm for smooth rate limiting with burst capacity.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnknownTelemetryRateLimitConfig {
    /// Maximum requests per second allowed from unknown/untrusted nodes.
    /// Set to 0 to disable rate limiting (allow unlimited requests).
    /// Default: 100 requests per second.
    #[serde(default = "default_unknown_rps")]
    pub requests_per_second: u32,

    /// Burst capacity - maximum number of requests that can be made in a burst.
    /// This allows short bursts above the sustained rate.
    /// Default: 200 requests (2x the per-second rate).
    #[serde(default = "default_unknown_burst")]
    pub burst_capacity: u32,

    /// Whether to enable rate limiting for unknown/untrusted nodes.
    /// Default: true.
    #[serde(default = "default_rate_limit_enabled")]
    pub enabled: bool,
}

fn default_unknown_rps() -> u32 {
    100 // 100 requests per second
}

fn default_unknown_burst() -> u32 {
    200 // 2x the per-second rate
}

fn default_rate_limit_enabled() -> bool {
    true
}

impl Default for UnknownTelemetryRateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: default_unknown_rps(),
            burst_capacity: default_unknown_burst(),
            enabled: default_rate_limit_enabled(),
        }
    }
}

impl TelemetryServiceConfig {
    pub fn load(path: PathBuf) -> Result<Self, anyhow::Error> {
        let mut file = File::open(&path).map_err(|e| {
            anyhow::anyhow!(
                "Unable to open file {}. Error: {}",
                path.to_str().unwrap(),
                e
            )
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            anyhow::anyhow!(
                "Unable to read file {}. Error: {}",
                path.to_str().unwrap(),
                e
            )
        })?;

        serde_yaml::from_str(&contents).map_err(|e| {
            anyhow::anyhow!(
                "Unable to read yaml {}. Error: {}",
                path.to_str().unwrap(),
                e
            )
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomEventConfig {
    pub project_id: String,
    pub dataset_id: String,
    pub table_id: String,
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    AptosTelemetryServiceArgs::command().debug_assert()
}

#[cfg(test)]
mod resolve_env_vars_tests {
    use super::OnChainAuthConfig;

    // SAFETY: These tests modify environment variables which is inherently unsafe
    // in multi-threaded contexts. Tests should be run with --test-threads=1 or
    // use unique variable names to avoid conflicts.

    #[test]
    fn test_basic_substitution() {
        // SAFETY: Test-only env var modification with unique name
        unsafe { std::env::set_var("TEST_VAR_BASIC", "hello") };
        let result = OnChainAuthConfig::resolve_env_vars("prefix_${TEST_VAR_BASIC}_suffix");
        assert_eq!(result.unwrap(), "prefix_hello_suffix");
        unsafe { std::env::remove_var("TEST_VAR_BASIC") };
    }

    #[test]
    fn test_multiple_substitutions() {
        // SAFETY: Test-only env var modification with unique names
        unsafe { std::env::set_var("TEST_VAR_A", "aaa") };
        unsafe { std::env::set_var("TEST_VAR_B", "bbb") };
        let result = OnChainAuthConfig::resolve_env_vars("${TEST_VAR_A}_${TEST_VAR_B}");
        assert_eq!(result.unwrap(), "aaa_bbb");
        unsafe { std::env::remove_var("TEST_VAR_A") };
        unsafe { std::env::remove_var("TEST_VAR_B") };
    }

    #[test]
    fn test_self_referential_no_infinite_loop() {
        // Set env var whose value contains ${VAR} pattern - should NOT be recursively expanded
        // SAFETY: Test-only env var modification with unique name
        unsafe { std::env::set_var("TEST_SELF_REF", "value_with_${TEST_SELF_REF}_inside") };
        let result = OnChainAuthConfig::resolve_env_vars("${TEST_SELF_REF}");
        // The inner ${TEST_SELF_REF} should NOT be substituted (prevents infinite loop)
        assert_eq!(result.unwrap(), "value_with_${TEST_SELF_REF}_inside");
        unsafe { std::env::remove_var("TEST_SELF_REF") };
    }

    #[test]
    fn test_cyclical_no_infinite_loop() {
        // Set up cyclical references: FOO -> ${BAR}, BAR -> ${FOO}
        // SAFETY: Test-only env var modification with unique names
        unsafe { std::env::set_var("TEST_CYCLE_FOO", "${TEST_CYCLE_BAR}") };
        unsafe { std::env::set_var("TEST_CYCLE_BAR", "${TEST_CYCLE_FOO}") };
        let result = OnChainAuthConfig::resolve_env_vars("${TEST_CYCLE_FOO}");
        // Should substitute once and stop - the value ${TEST_CYCLE_BAR} is NOT expanded
        assert_eq!(result.unwrap(), "${TEST_CYCLE_BAR}");
        unsafe { std::env::remove_var("TEST_CYCLE_FOO") };
        unsafe { std::env::remove_var("TEST_CYCLE_BAR") };
    }

    #[test]
    fn test_missing_env_var() {
        let result = OnChainAuthConfig::resolve_env_vars("${NONEXISTENT_VAR_12345}");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("NONEXISTENT_VAR_12345 not set"));
    }

    #[test]
    fn test_malformed_pattern() {
        let result = OnChainAuthConfig::resolve_env_vars("${UNCLOSED");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Malformed"));
    }

    #[test]
    fn test_no_substitution_needed() {
        let result = OnChainAuthConfig::resolve_env_vars("plain_string");
        assert_eq!(result.unwrap(), "plain_string");
    }
}

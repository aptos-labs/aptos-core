// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    clients::{big_query, victoria_metrics_api::Client as MetricsClient},
    context::{ClientTuple, Context, JsonWebTokenService, LogIngestClients, PeerStoreTuple},
    index::routes,
    metrics::PrometheusExporter,
    peer_location::PeerLocationUpdater,
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

        let bigquery_table_client = bigquery_client.as_ref().map(|client| {
            big_query::TableWriteClient::new(
                client.clone(),
                config.custom_event_config.project_id.clone(),
                config.custom_event_config.dataset_id.clone(),
                config.custom_event_config.table_id.clone(),
            )
        });

        let metrics_clients: GroupedMetricsClients = config.metrics_endpoints_config.clone().into();

        let telemetry_metrics_client = metrics_clients
            .telemetry_service_metrics_clients
            .values()
            .next()
            .cloned()
            .unwrap();

        let log_ingest_clients: LogIngestClients = config.humio_ingest_config.clone().into();

        // Setup custom contract clients from new config format
        let custom_contract_clients = if !config.custom_contract_configs.is_empty() {
            let mut instances = HashMap::new();

            for cc_config in &config.custom_contract_configs {
                let metrics_clients = cc_config
                    .metrics_sinks
                    .as_ref()
                    .map(|s| s.make_clients())
                    .unwrap_or_default();

                let logs_client = cc_config.logs_sink.as_ref().map(|s| s.make_client());

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

                instances.insert(cc_config.name.clone(), context::CustomContractInstance {
                    config: cc_config.on_chain_auth.clone(),
                    metrics_clients,
                    logs_client,
                    bigquery_client: cc_bigquery_client,
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

        let context = Context::new(
            server_private_key,
            PeerStoreTuple::new(
                validators.clone(),
                validator_fullnodes.clone(),
                public_fullnodes,
            ),
            ClientTuple::new(
                bigquery_table_client,
                Some(metrics_clients),
                Some(log_ingest_clients),
                custom_contract_clients,
            ),
            jwt_service,
            config.log_env_map.clone(),
            config.peer_identities.clone(),
            peer_locations.clone(),
            config.allowlist_cache_ttl_secs,
        );

        PeerSetCacheUpdater::new(
            validators,
            validator_fullnodes,
            config.trusted_full_node_addresses.clone(),
            Duration::from_secs(config.update_interval),
        )
        .run();

        // PeerLocationUpdater requires BigQuery - only start if available
        if let Some(bq_client) = bigquery_client.as_ref() {
            if let Err(err) =
                PeerLocationUpdater::new(bq_client.clone(), peer_locations.clone()).run()
            {
                error!("Failed to start PeerLocationUpdater: {:?}", err);
            }
        } else {
            warn!("PeerLocationUpdater disabled - BigQuery client not available");
        }

        PrometheusExporter::new(telemetry_metrics_client).run();

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

/// Per metric endpoint configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsEndpoint {
    /// Map of endpoint canonical name to Urls
    endpoint_urls: HashMap<String, Url>,
    /// Environment variable that holds the secrets
    keys_env_var: String,
}

impl MetricsEndpoint {
    #[cfg(test)]
    fn default_for_test() -> Self {
        Self {
            endpoint_urls: HashMap::new(),
            keys_env_var: "".into(),
        }
    }

    fn make_client(&self) -> HashMap<String, MetricsClient> {
        let secrets: HashMap<String, String> =
            serde_json::from_str(&env::var(&self.keys_env_var).unwrap_or_else(|_| {
                panic!(
                    "environment variable {} must be set and be a map of endpoint names to token",
                    self.keys_env_var.clone()
                )
            }))
            .unwrap_or_else(|_| {
                panic!(
                    "environment variable {} must be a map of name to secret",
                    self.keys_env_var
                )
            });

        self.endpoint_urls
            .iter()
            .map(|(name, url)| {
                let secret = secrets.get(name).unwrap_or_else(|| {
                    panic!(
                        "environment variable {} is missing secret for {}",
                        self.keys_env_var.clone(),
                        name
                    )
                });
                (name.clone(), MetricsClient::new(url.clone(), secret.into()))
            })
            .collect()
    }
}

/// Metrics endpoints configuration for metrics from
/// different datasources (node telemetry only)
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsEndpointsConfig {
    pub telemetry_service_metrics: MetricsEndpoint,
    pub ingest_metrics: MetricsEndpoint,
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

/// Metrics sink configuration (subset of MetricsEndpoint)
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsSinkConfig {
    /// Map of sink name to endpoint URL
    pub endpoint_urls: HashMap<String, String>,
    /// Environment variable containing JSON map of sink names to auth keys
    #[serde(default)]
    pub keys_env_var: Option<String>,
}

impl MetricsSinkConfig {
    /// Convert to MetricsClient instances
    pub fn make_clients(&self) -> HashMap<String, MetricsClient> {
        let keys: HashMap<String, String> = self
            .keys_env_var
            .as_ref()
            .and_then(|env_var| std::env::var(env_var).ok())
            .and_then(|json_str| serde_json::from_str(&json_str).ok())
            .unwrap_or_default();

        self.endpoint_urls
            .iter()
            .map(|(name, url)| {
                let secret: clients::victoria_metrics_api::AuthToken = keys
                    .get(name)
                    .map(|k| clients::victoria_metrics_api::AuthToken::Bearer(k.clone()))
                    .unwrap_or_else(|| {
                        clients::victoria_metrics_api::AuthToken::Bearer("".to_string())
                    });
                let parsed_url = Url::parse(url).expect("valid URL in metrics sink config");
                (name.clone(), MetricsClient::new(parsed_url, secret))
            })
            .collect()
    }
}

/// Custom contract configuration - consolidates auth and all data sinks
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomContractConfig {
    /// Unique identifier for this custom contract configuration
    /// Used in routing and logging
    pub name: String,

    /// On-chain authentication configuration
    pub on_chain_auth: OnChainAuthConfig,

    /// Metrics sinks for this custom contract (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_sinks: Option<MetricsSinkConfig>,

    /// Log sink for this custom contract (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs_sink: Option<LogIngestEndpoint>,

    /// BigQuery events sink for this custom contract (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events_sink: Option<CustomEventConfig>,
}

impl OnChainAuthConfig {
    /// Resolve environment variables in a string (${ENV_VAR} syntax)
    fn resolve_env_vars(input: &str) -> Result<String, String> {
        let mut resolved = input.to_string();

        // Find and replace ${ENV_VAR} patterns
        while let Some(start) = resolved.find("${") {
            if let Some(end) = resolved[start..].find('}') {
                let env_var = &resolved[start + 2..start + end];
                let value = std::env::var(env_var)
                    .map_err(|_| format!("Environment variable {} not set", env_var))?;
                resolved = format!(
                    "{}{}{}",
                    &resolved[..start],
                    value,
                    &resolved[start + end + 1..]
                );
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

/// A single log ingest endpoint config
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogIngestEndpoint {
    pub endpoint_url: Url,
    pub key_env_var: String,
    /// Log backend type (humio or loki). Defaults to humio for compatibility.
    #[serde(default)]
    pub backend_type: LogBackendType,
}

impl LogIngestEndpoint {
    #[cfg(test)]
    fn default_for_test() -> Self {
        Self {
            endpoint_url: Url::parse("test://test").unwrap(),
            key_env_var: "".into(),
            backend_type: LogBackendType::Humio,
        }
    }

    fn make_client(&self) -> context::LogIngestClient {
        use crate::clients::{humio, loki};

        let secret = env::var(&self.key_env_var).ok(); // Make optional for Loki

        match self.backend_type {
            LogBackendType::Humio => {
                let token = secret.unwrap_or_else(|| {
                    panic!(
                        "environment variable {} must be set for Humio backend.",
                        self.key_env_var.clone()
                    )
                });
                context::LogIngestClient::Humio(humio::IngestClient::new(
                    self.endpoint_url.clone(),
                    token,
                ))
            },
            LogBackendType::Loki => {
                // Loki auth token is optional
                context::LogIngestClient::Loki(loki::LokiIngestClient::new(
                    self.endpoint_url.clone(),
                    secret,
                ))
            },
        }
    }
}

/// Log ingest configuration for different sources
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogIngestConfig {
    // Log endpoint for known nodes (nodes from validator set, whitelist, etc.)
    pub known_logs_endpoint: LogIngestEndpoint,
    // Log endpoint for unknown nodes
    pub unknown_logs_endpoint: LogIngestEndpoint,
    // Blacklisted peers from log ingestion
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryServiceConfig {
    pub address: SocketAddr,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_cert_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_key_path: Option<String>,

    pub trusted_full_node_addresses: HashMap<ChainCommonName, String>,
    pub update_interval: u64,
    pub pfn_allowlist: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,

    pub custom_event_config: CustomEventConfig,
    pub humio_ingest_config: LogIngestConfig,

    pub log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
    pub peer_identities: HashMap<ChainId, HashMap<PeerId, String>>,

    pub metrics_endpoints_config: MetricsEndpointsConfig,

    /// Custom contract configurations (optional)
    /// Each entry defines authentication and data sinks for a different custom contract client type
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_contract_configs: Vec<CustomContractConfig>,

    /// Allowlist cache TTL in seconds (optional)
    /// Controls how long on-chain allowlist data is cached before re-fetching
    /// Default: 300 seconds (5 minutes). Set lower for testing (e.g., 10 seconds)
    #[serde(default = "default_allowlist_cache_ttl_secs")]
    pub allowlist_cache_ttl_secs: u64,
}

fn default_allowlist_cache_ttl_secs() -> u64 {
    300 // 5 minutes default
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

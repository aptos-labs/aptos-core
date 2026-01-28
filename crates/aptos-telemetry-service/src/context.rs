// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    allowlist_cache::AllowlistCache,
    challenge_cache::ChallengeCache,
    clients::{big_query::TableWriteClient, humio, prometheus_remote_write, victoria_metrics},
    peer_location::PeerLocation,
    types::common::EpochedPeerStore,
    LogIngestConfig, MetricsEndpointsConfig,
};
use aptos_crypto::{noise, x25519};
use aptos_infallible::RwLock;
use aptos_types::{chain_id::ChainId, PeerId};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, TokenData, Validation};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
};
use warp::Filter;

/// Metrics backend client - abstracts over VictoriaMetrics and Prometheus Remote Write
#[derive(Clone, Debug)]
pub enum MetricsIngestClient {
    /// VictoriaMetrics client - uses text format via /api/v1/import/prometheus
    VictoriaMetrics(victoria_metrics::VictoriaMetricsClient),
    /// Prometheus Remote Write client - uses protobuf+snappy via /api/v1/write
    PrometheusRemoteWrite(prometheus_remote_write::PrometheusRemoteWriteClient),
}

impl MetricsIngestClient {
    /// Posts Prometheus text-format metrics to the backend.
    ///
    /// - VictoriaMetrics: Sends text format directly
    /// - PrometheusRemoteWrite: Parses text, converts to protobuf+snappy
    pub async fn post_prometheus_metrics(
        &self,
        raw_metrics_body: warp::hyper::body::Bytes,
        extra_labels: Vec<String>,
        encoding: String,
    ) -> Result<reqwest::Response, anyhow::Error> {
        match self {
            MetricsIngestClient::VictoriaMetrics(client) => {
                client
                    .post_prometheus_metrics(raw_metrics_body, extra_labels, encoding)
                    .await
            },
            MetricsIngestClient::PrometheusRemoteWrite(client) => {
                client
                    .post_prometheus_metrics(raw_metrics_body, extra_labels, encoding)
                    .await
            },
        }
    }

    /// Returns true if this is a self-hosted VM/Prometheus client
    pub fn is_selfhosted_vm_client(&self) -> bool {
        match self {
            MetricsIngestClient::VictoriaMetrics(client) => client.is_selfhosted_vm_client(),
            MetricsIngestClient::PrometheusRemoteWrite(client) => client.is_selfhosted_vm_client(),
        }
    }

    /// Returns the backend name for logging/metrics
    pub fn backend_name(&self) -> &'static str {
        match self {
            MetricsIngestClient::VictoriaMetrics(_) => "victoria_metrics",
            MetricsIngestClient::PrometheusRemoteWrite(_) => "prometheus_remote_write",
        }
    }

    /// Returns the base URL for this client (for logging/debugging)
    pub fn base_url(&self) -> &url::Url {
        match self {
            MetricsIngestClient::VictoriaMetrics(client) => client.base_url(),
            MetricsIngestClient::PrometheusRemoteWrite(client) => client.base_url(),
        }
    }
}

/// Container that holds various metric clients used for sending metrics from
/// various sources to appropriate backends (node telemetry only).
#[derive(Clone, Default)]
pub struct GroupedMetricsClients {
    /// Client(s) for exporting metrics of the running telemetry service
    pub telemetry_service_metrics_clients: HashMap<String, MetricsIngestClient>,
    /// Clients for sending metrics from authenticated known nodes
    pub ingest_metrics_client: HashMap<String, MetricsIngestClient>,
    /// Clients for sending metrics from authenticated unknown nodes
    pub untrusted_ingest_metrics_clients: HashMap<String, MetricsIngestClient>,
}

impl GroupedMetricsClients {
    #[cfg(test)]
    pub fn new_empty() -> Self {
        Self {
            telemetry_service_metrics_clients: HashMap::new(),
            ingest_metrics_client: HashMap::new(),
            untrusted_ingest_metrics_clients: HashMap::new(),
        }
    }
}

impl From<MetricsEndpointsConfig> for GroupedMetricsClients {
    fn from(config: MetricsEndpointsConfig) -> GroupedMetricsClients {
        GroupedMetricsClients {
            telemetry_service_metrics_clients: config.telemetry_service_metrics.make_client(),
            ingest_metrics_client: config.ingest_metrics.make_client(),
            untrusted_ingest_metrics_clients: config.untrusted_ingest_metrics.make_client(),
        }
    }
}

/// Log backend type
#[derive(Clone, Debug)]
pub enum LogIngestClient {
    Humio(humio::IngestClient),
    Loki(crate::clients::loki::LokiIngestClient),
}

impl LogIngestClient {
    /// Ingest unstructured logs (works for both Humio and Loki)
    pub async fn ingest_unstructured_log(
        &self,
        log: crate::types::humio::UnstructuredLog,
    ) -> Result<reqwest::Response, anyhow::Error> {
        match self {
            LogIngestClient::Humio(client) => client.ingest_unstructured_log(log).await,
            LogIngestClient::Loki(client) => client.ingest_unstructured_log(log).await,
        }
    }
}

#[derive(Clone)]
pub struct LogIngestClients {
    pub known_logs_ingest_client: LogIngestClient,
    pub unknown_logs_ingest_client: LogIngestClient,
    pub blacklist: Option<HashSet<PeerId>>,
}

impl From<LogIngestConfig> for LogIngestClients {
    fn from(config: LogIngestConfig) -> Self {
        Self {
            known_logs_ingest_client: config.known_logs_endpoint.make_client(),
            unknown_logs_ingest_client: config.unknown_logs_endpoint.make_client(),
            blacklist: config.blacklist_peers,
        }
    }
}

#[derive(Clone, Default)]
pub struct PeerStoreTuple {
    validators: Arc<RwLock<EpochedPeerStore>>,
    validator_fullnodes: Arc<RwLock<EpochedPeerStore>>,
    public_fullnodes: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,
}

impl PeerStoreTuple {
    pub fn new(
        validators: Arc<RwLock<EpochedPeerStore>>,
        validator_fullnodes: Arc<RwLock<EpochedPeerStore>>,
        public_fullnodes: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,
    ) -> Self {
        Self {
            validators,
            validator_fullnodes,
            public_fullnodes,
        }
    }

    pub fn validators(&self) -> &Arc<RwLock<EpochedPeerStore>> {
        &self.validators
    }

    pub fn validator_fullnodes(&self) -> &Arc<RwLock<EpochedPeerStore>> {
        &self.validator_fullnodes
    }

    pub fn public_fullnodes(&self) -> &HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>> {
        &self.public_fullnodes
    }
}

/// Container for a single custom contract configuration and its clients
#[derive(Clone)]
pub struct CustomContractInstance {
    /// On-chain auth configuration (optional).
    /// When `None`, this is "open telemetry mode" - all nodes are treated as unknown.
    pub config: Option<crate::OnChainAuthConfig>,
    /// Custom node type name for labeling telemetry from this contract.
    /// Used in metrics labels as `node_type={node_type_name}`.
    pub node_type_name: String,
    /// Whether to allow unknown/untrusted nodes to authenticate via this contract
    pub allow_unknown_nodes: bool,
    /// Metrics clients for trusted (allowlisted) nodes
    pub metrics_clients: HashMap<String, MetricsIngestClient>,
    /// Metrics clients for untrusted/unknown nodes (falls back to metrics_clients if empty)
    pub untrusted_metrics_clients: HashMap<String, MetricsIngestClient>,
    /// Logs client for trusted (allowlisted) nodes
    pub logs_client: Option<LogIngestClient>,
    /// Logs client for untrusted/unknown nodes (falls back to logs_client if None)
    pub untrusted_logs_client: Option<LogIngestClient>,
    pub bigquery_client: Option<TableWriteClient>,
}

impl CustomContractInstance {
    /// Get the appropriate metrics clients based on whether the node is trusted
    pub fn get_metrics_clients(&self, is_trusted: bool) -> &HashMap<String, MetricsIngestClient> {
        if is_trusted || self.untrusted_metrics_clients.is_empty() {
            &self.metrics_clients
        } else {
            &self.untrusted_metrics_clients
        }
    }

    /// Get the appropriate logs client based on whether the node is trusted
    pub fn get_logs_client(&self, is_trusted: bool) -> Option<&LogIngestClient> {
        if is_trusted {
            self.logs_client.as_ref()
        } else {
            // Fall back to trusted logs client if untrusted not configured
            self.untrusted_logs_client
                .as_ref()
                .or(self.logs_client.as_ref())
        }
    }
}

/// Container for all custom contract configurations
#[derive(Clone, Default)]
pub struct CustomContractClients {
    /// Map of custom contract name to its instance
    pub instances: HashMap<String, CustomContractInstance>,
}

#[derive(Clone)]
pub struct ClientTuple {
    bigquery_client: Option<TableWriteClient>,
    victoria_metrics_clients: Option<GroupedMetricsClients>,
    log_ingest_clients: Option<LogIngestClients>,
    custom_contract_clients: Option<CustomContractClients>,
}

impl ClientTuple {
    pub(crate) fn new(
        bigquery_client: Option<TableWriteClient>,
        victoria_metrics_clients: Option<GroupedMetricsClients>,
        log_ingest_clients: Option<LogIngestClients>,
        custom_contract_clients: Option<CustomContractClients>,
    ) -> ClientTuple {
        Self {
            bigquery_client,
            victoria_metrics_clients,
            log_ingest_clients,
            custom_contract_clients,
        }
    }
}

#[derive(Clone)]
pub struct JsonWebTokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JsonWebTokenService {
    pub fn from_base64_secret(secret: &str) -> Self {
        let encoding_key = jsonwebtoken::EncodingKey::from_base64_secret(secret)
            .expect("jsonwebtoken key should be in base64 format.");
        let decoding_key = jsonwebtoken::DecodingKey::from_base64_secret(secret)
            .expect("jsonwebtoken key should be in base64 format.");
        Self {
            encoding_key,
            decoding_key,
        }
    }

    pub fn encode<T: Serialize>(&self, claims: T) -> Result<String, jsonwebtoken::errors::Error> {
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS512);
        jsonwebtoken::encode(&header, &claims, &self.encoding_key)
    }

    pub fn decode<T: DeserializeOwned>(
        &self,
        token: &str,
    ) -> Result<TokenData<T>, jsonwebtoken::errors::Error> {
        jsonwebtoken::decode::<T>(
            token,
            &self.decoding_key,
            &Validation::new(Algorithm::HS512),
        )
    }
}

#[derive(Clone)]
pub struct Context {
    noise_config: Arc<noise::NoiseConfig>,
    peers: PeerStoreTuple,
    clients: ClientTuple,
    jwt_service: JsonWebTokenService,
    log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
    peer_identities: HashMap<ChainId, HashMap<PeerId, String>>,
    peer_locations: Arc<RwLock<HashMap<PeerId, PeerLocation>>>,
    allowlist_cache: AllowlistCache,
    challenge_cache: ChallengeCache,
}

impl Context {
    pub fn new(
        private_key: x25519::PrivateKey,
        peers: PeerStoreTuple,
        clients: ClientTuple,
        jwt_service: JsonWebTokenService,
        log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
        peer_identities: HashMap<ChainId, HashMap<PeerId, String>>,
        peer_locations: Arc<RwLock<HashMap<PeerId, PeerLocation>>>,
    ) -> Self {
        Self {
            noise_config: Arc::new(noise::NoiseConfig::new(private_key)),
            peers,
            clients,
            jwt_service,
            log_env_map,
            peer_identities,
            peer_locations,
            // Cache is kept fresh by AllowlistCacheUpdater running in background
            allowlist_cache: AllowlistCache::new(),
            // Challenge cache uses same TTL as CHALLENGE_TTL_SECS (300 seconds)
            challenge_cache: ChallengeCache::new(),
        }
    }

    pub fn filter(self) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }

    pub fn noise_config(&self) -> Arc<noise::NoiseConfig> {
        self.noise_config.clone()
    }

    pub fn peers(&self) -> &PeerStoreTuple {
        &self.peers
    }

    pub fn jwt_service(&self) -> &JsonWebTokenService {
        &self.jwt_service
    }

    pub fn metrics_client(&self) -> &GroupedMetricsClients {
        self.clients.victoria_metrics_clients.as_ref().unwrap()
    }

    #[cfg(test)]
    pub fn metrics_client_mut(&mut self) -> &mut GroupedMetricsClients {
        self.clients.victoria_metrics_clients.as_mut().unwrap()
    }

    pub fn log_ingest_clients(&self) -> &LogIngestClients {
        self.clients.log_ingest_clients.as_ref().unwrap()
    }

    pub(crate) fn bigquery_client(&self) -> Option<&TableWriteClient> {
        self.clients.bigquery_client.as_ref()
    }

    pub(crate) fn peer_identities(&self) -> &HashMap<ChainId, HashMap<PeerId, String>> {
        &self.peer_identities
    }

    pub(crate) fn peer_locations(&self) -> &Arc<RwLock<HashMap<PeerId, PeerLocation>>> {
        &self.peer_locations
    }

    pub fn chain_set(&self) -> HashSet<ChainId> {
        self.peers.validators.read().keys().cloned().collect()
    }

    pub fn log_env_map(&self) -> &HashMap<ChainId, HashMap<PeerId, String>> {
        &self.log_env_map
    }

    #[cfg(test)]
    pub fn log_env_map_mut(&mut self) -> &mut HashMap<ChainId, HashMap<PeerId, String>> {
        &mut self.log_env_map
    }

    /// Get storage provider clients
    pub fn custom_contract_clients(&self) -> &CustomContractClients {
        self.clients
            .custom_contract_clients
            .as_ref()
            .unwrap_or_else(|| {
                // Return empty clients if not configured
                static EMPTY: once_cell::sync::Lazy<CustomContractClients> =
                    once_cell::sync::Lazy::new(CustomContractClients::default);
                &EMPTY
            })
    }

    /// Get a specific custom contract instance by name
    pub fn get_custom_contract(&self, name: &str) -> Option<&CustomContractInstance> {
        self.custom_contract_clients().instances.get(name)
    }

    /// Get the allowlist cache
    pub fn allowlist_cache(&self) -> &AllowlistCache {
        &self.allowlist_cache
    }

    /// Get the challenge cache for custom contract authentication
    pub fn challenge_cache(&self) -> &ChallengeCache {
        &self.challenge_cache
    }
}

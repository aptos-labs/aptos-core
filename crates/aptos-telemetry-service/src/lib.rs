// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    clients::{big_query, humio, victoria_metrics_api::Client as MetricsClient},
    context::{ClientTuple, Context, JsonWebTokenService, PeerStoreTuple},
    index::routes,
    metrics::PrometheusExporter,
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
    collections::HashMap, convert::Infallible, env, fs::File, io::Read, net::SocketAddr,
    path::PathBuf, sync::Arc, time::Duration,
};
use warp::{Filter, Reply};

mod auth;
mod clients;
mod constants;
mod context;
mod custom_event;
mod errors;
mod gcp_logger;
mod index;
mod jwt_auth;
mod log_ingest;
mod metrics;
mod prometheus_push_metrics;
mod remote_config;
#[cfg(any(test))]
pub(crate) mod tests;
pub mod types;
mod validator_cache;

#[derive(Clone, Debug, Parser)]
#[clap(name = "Aptos Telemetry Service", author, version)]
pub struct AptosTelemetryServiceArgs {
    #[clap(short = 'f', long, parse(from_os_str))]
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

        let bigquery_client = BigQueryClient::from_service_account_key_file(
            env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .expect("environment variable GOOGLE_APPLICATION_CREDENTIALS must be set")
                .as_str(),
        )
        .await;
        let bigquery_client = big_query::TableWriteClient::new(
            bigquery_client,
            config.custom_event_config.project_id.clone(),
            config.custom_event_config.dataset_id.clone(),
            config.custom_event_config.table_id.clone(),
        );

        let metrics_clients: GroupedMetricsClients = config.metrics_endpoints_config.clone().into();

        let telemetry_metrics_client = metrics_clients
            .telemetry_service_metrics_clients
            .values()
            .next()
            .cloned()
            .unwrap();

        let humio_client = humio::IngestClient::new(
            Url::parse(&config.humio_url).expect("invalid Humio ingest endpoint URL"),
            env::var("HUMIO_INGEST_TOKEN")
                .expect("environment variable HUMIO_INGEST_TOKEN must be set"),
        );

        let jwt_service = JsonWebTokenService::from_base64_secret(
            env::var("JWT_SIGNING_KEY")
                .expect("environment variable JWT_SIGNING_KEY must be set")
                .as_str(),
        );

        let validators = Arc::new(aptos_infallible::RwLock::new(HashMap::new()));
        let validator_fullnodes = Arc::new(aptos_infallible::RwLock::new(HashMap::new()));
        let public_fullnodes = config.pfn_allowlist.clone();
        let chain_set = config
            .trusted_full_node_addresses
            .iter()
            .map(|pair| pair.0.to_owned())
            .collect();

        let context = Context::new(
            server_private_key,
            PeerStoreTuple::new(
                validators.clone(),
                validator_fullnodes.clone(),
                public_fullnodes,
            ),
            ClientTuple::new(
                Some(bigquery_client),
                Some(metrics_clients),
                Some(humio_client),
            ),
            chain_set,
            jwt_service,
            config.log_env_map.clone(),
            config.peer_identities.clone(),
        );

        PeerSetCacheUpdater::new(
            validators,
            validator_fullnodes,
            config.trusted_full_node_addresses.clone(),
            Duration::from_secs(config.update_interval),
        )
        .run();

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
                        name,
                    )
                });
                (
                    name.clone(),
                    MetricsClient::new(url.clone(), secret.clone()),
                )
            })
            .collect()
    }
}

/// Metrics endpoints configuration for metrics from
/// different datasources.
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryServiceConfig {
    pub address: SocketAddr,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_cert_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_key_path: Option<String>,

    pub trusted_full_node_addresses: HashMap<ChainId, String>,
    pub update_interval: u64,
    pub pfn_allowlist: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,

    pub custom_event_config: CustomEventConfig,
    pub humio_url: String,

    pub log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
    pub peer_identities: HashMap<ChainId, HashMap<PeerId, String>>,

    pub metrics_endpoints_config: MetricsEndpointsConfig,
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

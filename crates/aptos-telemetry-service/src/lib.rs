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
use gcp_bigquery_client::Client as BigQueryClient;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
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

        let victoria_metrics_secrets: HashMap<String, String> = serde_json::from_str(
            &env::var("VICTORIA_METRICS_AUTH_TOKEN")
                .expect("environment variable VICTORIA_METRICS_AUTH_TOKEN must be set"),
        )
        .expect("environment variable VICTORIA_METRICS_AUTH_TOKEN must be a map of name to secret");

        let victoria_metrics_clients: BTreeMap<String, MetricsClient> = config
            .victoria_metrics_endpoints
            .iter()
            .map(|(name, url)| {
                let secret = victoria_metrics_secrets.get(name).unwrap_or_else(|| {
                    panic!(
                        "environment variable VICTORIA_METRICS_AUTH_TOKEN is missing secret for {}",
                        name
                    )
                });
                (
                    name.clone(),
                    MetricsClient::new(
                        Url::parse(url).unwrap_or_else(|e| {
                            panic!("invalid metrics ingest endpoint URL for {}: {}", name, e)
                        }),
                        secret.clone(),
                    ),
                )
            })
            .collect();

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

        let context = Context::new(
            server_private_key,
            PeerStoreTuple::new(
                validators.clone(),
                validator_fullnodes.clone(),
                public_fullnodes,
            ),
            ClientTuple::new(
                Some(bigquery_client),
                Some(victoria_metrics_clients),
                Some(humio_client),
            ),
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

        let metrics_exporter_client = MetricsClient::new(
            Url::parse(&config.metrics_exporter_base_url)
                .expect("base url must be provided for victoria metrics exporter url"),
            env::var("METRICS_EXPORTER_AUTH_TOKEN")
                .expect("environment variable METRICS_EXPORTER_AUTH_TOKEN must be set"),
        );

        PrometheusExporter::new(metrics_exporter_client).run();

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
    pub victoria_metrics_endpoints:
        HashMap<String /* endpoint name */, String /* endpoint Url */>,
    pub metrics_exporter_base_url: String,
    pub humio_url: String,

    pub log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
    pub peer_identities: HashMap<ChainId, HashMap<PeerId, String>>,
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

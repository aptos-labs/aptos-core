// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap, convert::Infallible, env, fs::File, io::Read, net::SocketAddr,
    path::PathBuf, sync::Arc,
};

use aptos_config::keys::ConfigKey;
use aptos_crypto::x25519;
use aptos_types::{chain_id::ChainId, PeerId};
use clap::Parser;
use gcp_bigquery_client::Client;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing::info;
use warp::{Filter, Reply};

use crate::{
    clients::humio,
    clients::victoria_metrics_api::Client as MetricsClient,
    context::Context,
    index::routes,
    validator_cache::{PeerSetCache, PeerSetCacheUpdater},
};

mod auth;
mod clients;
mod constants;
mod context;
mod custom_event;
mod error;
mod index;
mod jwt_auth;
mod log_ingest;
mod prometheus_push_metrics;
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

        let gcp_bigquery_client = Client::from_service_account_key_file(
            env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .expect("environment variable GOOGLE_APPLICATION_CREDENTIALS must be set")
                .as_str(),
        )
        .await;

        let victoria_metrics_client = MetricsClient::new(
            Url::parse(&config.victoria_metrics_base_url)
                .expect("base url must be provided for victoria metrics"),
            config.victoria_metrics_token.clone(),
        );

        let humio_client = humio::IngestClient::new(
            Url::parse(&config.humio_url).unwrap(),
            config.humio_auth_token.clone(),
        );
        let validators_cache = PeerSetCache::new(aptos_infallible::RwLock::new(HashMap::new()));
        let vfns_cache = PeerSetCache::new(aptos_infallible::RwLock::new(HashMap::new()));
        let pfns_cache = Arc::new(aptos_infallible::RwLock::new(HashMap::new()));

        pfns_cache.write().clone_from(&config.pfn_allowlist);

        let context = Context::new(
            &config,
            validators_cache.clone(),
            vfns_cache.clone(),
            pfns_cache.clone(),
            Some(gcp_bigquery_client),
            Some(victoria_metrics_client),
            humio_client,
        );

        PeerSetCacheUpdater::new(validators_cache, vfns_cache, &config).run();

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
            }
        };
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryServiceConfig {
    pub address: SocketAddr,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_cert_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_key_path: Option<String>,
    pub trusted_full_node_addresses: HashMap<ChainId, String>,
    pub server_private_key: ConfigKey<x25519::PrivateKey>,
    pub jwt_signing_key: String,
    pub update_interval: u64,
    pub gcp_bq_config: GCPBigQueryConfig,
    pub victoria_metrics_base_url: String,
    pub victoria_metrics_token: String,
    pub humio_url: String,
    pub humio_auth_token: String,
    pub pfn_allowlist: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,
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
pub struct GCPBigQueryConfig {
    pub project_id: String,
    pub dataset_id: String,
    pub table_id: String,
}

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::{convert::Infallible, sync::Arc};

use crate::validator_cache::PeerSetCache;
use crate::{
    clients::humio, clients::victoria_metrics_api::Client as MetricsClient, GCPBigQueryConfig,
    TelemetryServiceConfig,
};
use aptos_crypto::{noise, x25519};
use aptos_types::chain_id::ChainId;
use aptos_types::PeerId;
use gcp_bigquery_client::Client as BQClient;
use jsonwebtoken::{DecodingKey, EncodingKey};
use warp::Filter;

#[derive(Clone)]
pub struct Context {
    noise_config: Arc<noise::NoiseConfig>,
    validator_cache: PeerSetCache,
    vfn_cache: PeerSetCache,
    pfn_cache: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,

    configured_chains: HashSet<ChainId>,
    log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,

    pub gcp_bq_client: Option<BQClient>,
    pub gcp_bq_config: GCPBigQueryConfig,

    pub victoria_metrics_client: Option<MetricsClient>,

    pub jwt_encoding_key: EncodingKey,
    pub jwt_decoding_key: DecodingKey,

    pub humio_client: humio::IngestClient,
}

impl Context {
    pub fn new(
        config: &TelemetryServiceConfig,
        validator_cache: PeerSetCache,
        vfn_cache: PeerSetCache,
        pfn_cache: HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>>,
        log_env_map: HashMap<ChainId, HashMap<PeerId, String>>,
        gcp_bigquery_client: Option<BQClient>,
        victoria_metrics_client: Option<MetricsClient>,
        humio_client: humio::IngestClient,
    ) -> Self {
        let private_key = config.server_private_key.private_key();
        let configured_chains = config
            .trusted_full_node_addresses
            .iter()
            .map(|(chain_id, _)| *chain_id)
            .collect();
        Self {
            noise_config: Arc::new(noise::NoiseConfig::new(private_key)),
            validator_cache,
            vfn_cache,
            pfn_cache,
            configured_chains,
            log_env_map,

            gcp_bq_client: gcp_bigquery_client,
            gcp_bq_config: config.gcp_bq_config.clone(),

            victoria_metrics_client,

            jwt_encoding_key: EncodingKey::from_secret(config.jwt_signing_key.as_bytes()),
            jwt_decoding_key: DecodingKey::from_secret(config.jwt_signing_key.as_bytes()),

            humio_client,
        }
    }

    pub fn filter(self) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }

    pub fn validator_cache(&self) -> PeerSetCache {
        self.validator_cache.clone()
    }

    pub fn vfn_cache(&self) -> PeerSetCache {
        self.vfn_cache.clone()
    }

    pub fn pfn_cache(&self) -> &HashMap<ChainId, HashMap<PeerId, x25519::PublicKey>> {
        &self.pfn_cache
    }

    pub fn noise_config(&self) -> Arc<noise::NoiseConfig> {
        self.noise_config.clone()
    }

    pub fn configured_chains(&self) -> &HashSet<ChainId> {
        &self.configured_chains
    }

    pub fn log_env_map(&self) -> &HashMap<ChainId, HashMap<PeerId, String>> {
        &self.log_env_map
    }

    #[cfg(test)]
    pub fn log_env_map_mut(&mut self) -> &mut HashMap<ChainId, HashMap<PeerId, String>> {
        &mut self.log_env_map
    }

    #[cfg(test)]
    pub fn configured_chains_mut(&mut self) -> &mut HashSet<ChainId> {
        &mut self.configured_chains
    }
}

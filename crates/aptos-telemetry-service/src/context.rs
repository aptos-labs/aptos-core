// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{convert::Infallible, sync::Arc};

use crate::validator_cache::PeerSetCache;
use crate::{
    clients::humio, clients::victoria_metrics_api::Client as MetricsClient, GCPBigQueryConfig,
    TelemetryServiceConfig,
};
use aptos_crypto::noise;
use gcp_bigquery_client::Client as BQClient;
use jsonwebtoken::{DecodingKey, EncodingKey};
use warp::Filter;

#[derive(Clone)]
pub struct Context {
    noise_config: Arc<noise::NoiseConfig>,
    validator_cache: PeerSetCache,
    vfn_cache: PeerSetCache,

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
        gcp_bigquery_client: Option<BQClient>,
        victoria_metrics_client: Option<MetricsClient>,
        humio_client: humio::IngestClient,
    ) -> Self {
        let private_key = config.server_private_key.private_key();
        Self {
            noise_config: Arc::new(noise::NoiseConfig::new(private_key)),
            validator_cache,
            vfn_cache,

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

    pub fn noise_config(&self) -> Arc<noise::NoiseConfig> {
        self.noise_config.clone()
    }
}

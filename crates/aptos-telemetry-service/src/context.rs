// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{convert::Infallible, sync::Arc};

use crate::{validator_cache::ValidatorSetCache, AptosTelemetryServiceConfig};
use aptos_crypto::noise;
use jsonwebtoken::{DecodingKey, EncodingKey};
use warp::Filter;

#[derive(Clone)]
pub struct Context {
    noise_config: Arc<noise::NoiseConfig>,
    validator_cache: ValidatorSetCache,

    pub jwt_encoding_key: EncodingKey,
    pub jwt_decoding_key: DecodingKey,
}

impl Context {
    pub fn new(config: &AptosTelemetryServiceConfig, validator_cache: ValidatorSetCache) -> Self {
        let private_key = config.server_private_key.private_key();
        Self {
            noise_config: Arc::new(noise::NoiseConfig::new(private_key)),
            validator_cache,

            jwt_encoding_key: EncodingKey::from_secret(config.jwt_signing_key.as_bytes()),
            jwt_decoding_key: DecodingKey::from_secret(config.jwt_signing_key.as_bytes()),
        }
    }

    pub fn filter(self) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }

    pub fn validator_cache(&self) -> ValidatorSetCache {
        self.validator_cache.clone()
    }

    pub fn noise_config(&self) -> Arc<noise::NoiseConfig> {
        self.noise_config.clone()
    }
}

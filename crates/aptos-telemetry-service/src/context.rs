use std::{convert::Infallible, sync::Arc};

use crate::{validator_cache::ValidatorSetCache, AptosTelemetryServiceConfig, auth::JWTAuthentication};
use aptos_crypto::noise;
use jsonwebtoken::EncodingKey;
use warp::Filter;

#[derive(Clone)]
pub struct Context {
    jwt_auth: JWTAuthentication,
    noise_config: Arc<noise::NoiseConfig>,
    validator_cache: ValidatorSetCache,
}

impl Context {
    pub fn new(config: &AptosTelemetryServiceConfig, validator_cache: ValidatorSetCache) -> Self {
        let private_key = config.server_private_key.private_key();
        Self {
            jwt_auth: JWTAuthentication::new(EncodingKey::from_secret(config.jwt_signing_key.as_bytes())),
            noise_config: Arc::new(noise::NoiseConfig::new(private_key)),
            validator_cache,
        }
    }

    pub fn filter(self) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }

    pub fn validator_cache(&self) -> ValidatorSetCache {
        return self.validator_cache.clone();
    }

    pub fn noise_config(&self) -> Arc<noise::NoiseConfig> {
        return self.noise_config.clone();
    }

    pub fn jwt_auth(&self) -> &JWTAuthentication {
        return &self.jwt_auth
    }
}

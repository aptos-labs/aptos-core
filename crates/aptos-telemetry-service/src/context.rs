use std::{convert::Infallible, sync::Arc};

use crate::validator_cache::ValidatorCache;
use aptos_crypto::{x25519, noise};
use warp::Filter;

#[derive(Clone)]
pub struct Context {
    noise_config: Arc<noise::NoiseConfig>,
    validator_cache: ValidatorCache,
}

impl Context {
    pub fn new(private_key: x25519::PrivateKey, validator_cache: ValidatorCache) -> Self {
        Self {
            noise_config: Arc::new(noise::NoiseConfig::new(private_key)),
            validator_cache,
        }
    }

    pub fn filter(self) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }

    pub fn validator_cache(&self) -> ValidatorCache {
        return self.validator_cache.clone();
    }

    pub(crate) fn noise_config(&self) -> Arc<noise::NoiseConfig> {
        return self.noise_config.clone();
    }
}

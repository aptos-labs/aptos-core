// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::environment::AptosEnvironment;
use aptos_types::state_store::StateView;
use bytes::Bytes;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// Represents a unique identifier for an [AptosEnvironment] instance based on the features, gas
/// feature version, and other configs.
#[derive(Hash, Eq, PartialEq)]
struct EnvironmentID {
    bytes: Bytes,
}

impl EnvironmentID {
    /// Create a new identifier for the given environment.
    fn new(env: &AptosEnvironment) -> Self {
        // These are sufficient to distinguish different environments.
        let chain_id = env.chain_id();
        let features = env.features();
        let timed_features = env.timed_features();
        let gas_feature_version = env.gas_feature_version();
        let vm_config = env.vm_config();

        let bytes = bcs::to_bytes(&(
            chain_id,
            features,
            timed_features,
            gas_feature_version,
            vm_config,
        ))
        .expect("Should be able to serialize all configs")
        .into();
        Self { bytes }
    }
}

/// A cache of different environment that can be persisted across blocks. Used by block executor
/// only.
pub struct EnvironmentCache(Mutex<lru::LruCache<EnvironmentID, AptosEnvironment>>);

impl EnvironmentCache {
    const CACHE_SIZE: usize = 32;

    /// Returns the cached environment if it exists and has the same configuration as if it was
    /// created based on the current state, or creates a new one and caches it. Should only be
    /// called at the block boundaries.
    pub fn fetch_with_delayed_field_optimization_enabled(
        state_view: &impl StateView,
    ) -> AptosEnvironment {
        let env = AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);
        let id = EnvironmentID::new(&env);
        ENVIRONMENT_CACHE.get_or_fetch(id, env)
    }

    /// Returns new environment cache.
    fn empty() -> Self {
        Self(Mutex::new(lru::LruCache::new(Self::CACHE_SIZE)))
    }

    /// Returns the newly created environment, or the cached one.
    fn get_or_fetch(&self, id: EnvironmentID, env: AptosEnvironment) -> AptosEnvironment {
        let mut cache = self.0.lock();
        if let Some(cached_env) = cache.get(&id) {
            return cached_env.clone();
        }

        cache.push(id, env.clone());
        env
    }
}

/// Long-living environment cache to be used across blocks.
static ENVIRONMENT_CACHE: Lazy<EnvironmentCache> = Lazy::new(EnvironmentCache::empty);

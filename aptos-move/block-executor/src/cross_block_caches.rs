// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::StateView;
use aptos_vm_environment::environment::AptosEnvironment;
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

/// A cached environment that can be persisted across blocks. Used by block executor only. Also
/// stores an identifier so that we can check when it changes.
pub struct CachedAptosEnvironment {
    id: EnvironmentID,
    env: AptosEnvironment,
}

impl CachedAptosEnvironment {
    /// Returns the cached environment if it exists and has the same configuration as if it was
    /// created based on the current state, or creates a new one and caches it. Should only be
    /// called at the block boundaries.
    pub fn fetch_with_delayed_field_optimization_enabled(
        state_view: &impl StateView,
    ) -> AptosEnvironment {
        // Create a new environment.
        let env = AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);
        let id = EnvironmentID::new(&env);

        // Lock the cache, and check if the environment is the same.
        let mut cross_block_environment = CROSS_BLOCK_ENVIRONMENT.lock();
        if let Some(cached_env) = cross_block_environment.as_ref() {
            if id == cached_env.id {
                return cached_env.env.clone();
            }
        }

        // It is not, so we have to reset it.
        *cross_block_environment = Some(CachedAptosEnvironment {
            id,
            env: env.clone(),
        });
        drop(cross_block_environment);

        env
    }
}

static CROSS_BLOCK_ENVIRONMENT: Lazy<Mutex<Option<CachedAptosEnvironment>>> =
    Lazy::new(|| Mutex::new(None));

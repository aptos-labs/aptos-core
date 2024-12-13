// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Defines different overrides for on-chain state used for benchmarking. With overrides, past
//! transactions can be replayed on top of a modified state, and we can evaluate how it impacts
//! performance or other things.

use aptos_logger::error;
use aptos_types::{
    on_chain_config::{FeatureFlag, Features, OnChainConfig},
    state_store::{state_key::StateKey, state_value::StateValue, StateView},
};
use serde::Serialize;
use std::collections::HashMap;

/// Stores feature flags to enable/disable, essentially overriding on-chain state.
pub struct OverrideConfig {
    additional_enabled_features: Vec<FeatureFlag>,
    additional_disabled_features: Vec<FeatureFlag>,
}

impl OverrideConfig {
    pub fn new(
        additional_enabled_features: Vec<FeatureFlag>,
        additional_disabled_features: Vec<FeatureFlag>,
    ) -> Self {
        Self {
            additional_enabled_features,
            additional_disabled_features,
        }
    }

    pub(crate) fn get_state_override(
        &self,
        state_view: &impl StateView,
    ) -> HashMap<StateKey, StateValue> {
        let mut state_override = HashMap::new();

        // Enable/disable features.
        let (features_state_key, features_state_value) =
            config_override::<Features, _>(state_view, |features| {
                for feature in &self.additional_enabled_features {
                    if features.is_enabled(*feature) {
                        error!("Feature {:?} is already enabled", feature);
                    }
                    features.enable(*feature);
                }
                for feature in &self.additional_disabled_features {
                    if !features.is_enabled(*feature) {
                        error!("Feature {:?} is already disabled", feature);
                    }
                    features.disable(*feature);
                }
            });
        state_override.insert(features_state_key, features_state_value);
        state_override
    }
}

/// Returns the state key for on-chain config type.
fn config_state_key<T: OnChainConfig>() -> StateKey {
    StateKey::resource(T::address(), &T::struct_tag())
        .expect("Constructing state key for on-chain config must succeed")
}

/// Fetches the config from the storage, and modifies it based on the passed function. Panics if
/// there is a storage error, config does not exist or fails to (de-)serialize.
fn config_override<T: OnChainConfig + Serialize, F: FnOnce(&mut T)>(
    state_view: &impl StateView,
    override_func: F,
) -> (StateKey, StateValue) {
    let state_key = config_state_key::<T>();
    let state_value = state_view
        .get_state_value(&state_key)
        .unwrap_or_else(|err| {
            panic!(
                "Failed to fetch on-chain config for {:?}: {:?}",
                state_key, err
            )
        })
        .unwrap_or_else(|| panic!("On-chain config for {:?} must always exist", state_key));

    let mut config = T::deserialize_into_config(state_value.bytes())
        .expect("On-chain config must be deserializable");
    override_func(&mut config);
    let config_bytes = bcs::to_bytes(&config).expect("On-chain config must be serializable");

    let new_state_value = state_value.map_bytes(|_| Ok(config_bytes.into())).unwrap();
    (state_key, new_state_value)
}

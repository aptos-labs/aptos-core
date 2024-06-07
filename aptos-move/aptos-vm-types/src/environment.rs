// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{
        ConfigurationResource, Features, OnChainConfig, TimedFeatureOverride, TimedFeatures,
        TimedFeaturesBuilder,
    },
    state_store::StateView,
    vm::configs::{aptos_prod_vm_config, get_timed_feature_override},
};
use move_vm_runtime::config::VMConfig;
use std::sync::Arc;

/// A runtime environment which can be used for VM initialization and more.
#[derive(Clone)]
pub struct Environment {
    pub chain_id: ChainId,

    pub features: Features,
    pub timed_features: TimedFeatures,

    pub vm_config: VMConfig,
}

impl Environment {
    pub fn new(state_view: &impl StateView) -> Self {
        let features = Features::fetch_config(state_view).unwrap_or_default();

        // If no chain ID is in storage, we assume we are in a testing environment.
        let chain_id = ChainId::fetch_config(state_view).unwrap_or_else(ChainId::test);
        let timestamp = ConfigurationResource::fetch_config(state_view)
            .map(|config| config.last_reconfiguration_time())
            .unwrap_or(0);

        let mut timed_features_builder = TimedFeaturesBuilder::new(chain_id, timestamp);
        if let Some(profile) = get_timed_feature_override() {
            timed_features_builder = timed_features_builder.with_override_profile(profile)
        }
        let timed_features = timed_features_builder.build();

        Self::initialize(features, timed_features, chain_id)
    }

    pub fn genesis() -> Arc<Self> {
        let chain_id = ChainId::test();
        let features = Features::default();
        let timed_features = TimedFeaturesBuilder::enable_all().build();
        // Wrap as Arc here to simplify the code in callers.
        Arc::new(Self::initialize(features, timed_features, chain_id))
    }

    pub fn testing(chain_id: ChainId) -> Arc<Self> {
        let features = Features::default();
        let timed_features = TimedFeaturesBuilder::enable_all()
            .with_override_profile(TimedFeatureOverride::Testing)
            .build();
        // Wrap as Arc here to simplify the code in tests.
        Arc::new(Self::initialize(features, timed_features, chain_id))
    }

    pub fn with_features_for_testing(self, features: Features) -> Arc<Self> {
        // We need to re-initialize configs because they depend on the feature flags!
        Arc::new(Self::initialize(
            features,
            self.timed_features,
            self.chain_id,
        ))
    }

    pub fn try_enable_delayed_field_optimization(mut self) -> Self {
        if self.features.is_aggregator_v2_delayed_fields_enabled() {
            self.vm_config.delayed_field_optimization_enabled = true;
        }
        self
    }

    fn initialize(features: Features, timed_features: TimedFeatures, chain_id: ChainId) -> Self {
        // By default, do not use delayed field optimization. Instead, clients should enable it
        // manually where applicable.
        let delayed_field_optimization_enabled = false;

        let vm_config = aptos_prod_vm_config(
            &features,
            &timed_features,
            delayed_field_optimization_enabled,
        );

        Self {
            chain_id,
            features,
            timed_features,
            vm_config,
        }
    }
}

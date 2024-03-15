// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::{gas_feature_versions::RELEASE_V1_15, AptosGasParameters};
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
use move_vm_types::loaded_data::runtime_types::TypeBuilder;
use std::sync::Arc;

// TODO(George): move configs here from types crate.
pub fn aptos_prod_ty_builder(
    gas_feature_version: u64,
    gas_params: &AptosGasParameters,
) -> TypeBuilder {
    if gas_feature_version >= RELEASE_V1_15 {
        let max_ty_size = gas_params.vm.txn.max_ty_size;
        let max_ty_depth = gas_params.vm.txn.max_ty_depth;
        TypeBuilder::with_limits(max_ty_size.into(), max_ty_depth.into())
    } else {
        aptos_default_ty_builder()
    }
}

pub fn aptos_default_ty_builder() -> TypeBuilder {
    // Type builder to use when:
    //   1. Type size gas parameters are not yet in gas schedule (before 1.15).
    //   2. No gas parameters are found on-chain.
    TypeBuilder::with_limits(128, 20)
}

/// A runtime environment which can be used for VM initialization and more.
#[derive(Clone)]
pub struct Environment {
    chain_id: ChainId,

    features: Features,
    timed_features: TimedFeatures,

    vm_config: VMConfig,
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

        let ty_builder = aptos_default_ty_builder();
        Self::initialize(features, timed_features, chain_id, ty_builder)
    }

    pub fn testing(chain_id: ChainId) -> Arc<Self> {
        let features = Features::default();

        // FIXME: should probably read the timestamp from storage.
        let timed_features = TimedFeaturesBuilder::enable_all()
            .with_override_profile(TimedFeatureOverride::Testing)
            .build();

        let ty_builder = aptos_default_ty_builder();
        Arc::new(Self::initialize(
            features,
            timed_features,
            chain_id,
            ty_builder,
        ))
    }

    pub fn with_features_for_testing(self, features: Features) -> Arc<Self> {
        let ty_builder = aptos_default_ty_builder();
        Arc::new(Self::initialize(
            features,
            self.timed_features,
            self.chain_id,
            ty_builder,
        ))
    }

    pub fn try_enable_delayed_field_optimization(mut self) -> Self {
        if self.features.is_aggregator_v2_delayed_fields_enabled() {
            self.vm_config.delayed_field_optimization_enabled = true;
        }
        self
    }

    #[inline]
    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    #[inline]
    pub fn features(&self) -> &Features {
        &self.features
    }

    #[inline]
    pub fn timed_features(&self) -> &TimedFeatures {
        &self.timed_features
    }

    #[inline]
    pub fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    fn initialize(
        features: Features,
        timed_features: TimedFeatures,
        chain_id: ChainId,
        ty_builder: TypeBuilder,
    ) -> Self {
        let vm_config = aptos_prod_vm_config(&features, &timed_features, ty_builder);

        Self {
            chain_id,
            features,
            timed_features,
            vm_config,
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use aptos_language_e2e_tests::data_store::FakeDataStore;

    #[test]
    fn test_new_environment() {
        // This creates an empty state.
        let state_view = FakeDataStore::default();
        let env = Environment::new(&state_view);

        // Check default values.
        assert_eq!(&env.features, &Features::default());
        assert_eq!(env.chain_id.id(), ChainId::test().id());
        assert!(!env.vm_config.delayed_field_optimization_enabled);

        let env = env.try_enable_delayed_field_optimization();
        assert!(env.vm_config.delayed_field_optimization_enabled);
    }

    #[test]
    fn test_environment_for_testing() {
        let env = Environment::testing(ChainId::new(55));

        assert_eq!(&env.features, &Features::default());
        assert_eq!(env.chain_id.id(), 55);
        assert!(!env.vm_config.delayed_field_optimization_enabled);

        let expected_timed_features = TimedFeaturesBuilder::enable_all()
            .with_override_profile(TimedFeatureOverride::Testing)
            .build();
        assert_eq!(&env.timed_features, &expected_timed_features);
    }
}

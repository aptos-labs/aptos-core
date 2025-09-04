// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    gas::get_gas_parameters,
    natives::velor_natives_with_builder,
    prod_configs::{
        velor_default_ty_builder, velor_prod_ty_builder, velor_prod_vm_config,
        get_timed_feature_override,
    },
};
use velor_gas_algebra::DynamicExpression;
use velor_gas_schedule::{VelorGasParameters, MiscGasParameters, NativeGasParameters};
use velor_native_interface::SafeNativeBuilder;
use velor_types::{
    chain_id::ChainId,
    on_chain_config::{
        ConfigurationResource, Features, OnChainConfig, TimedFeatures, TimedFeaturesBuilder,
    },
    state_store::StateView,
};
use velor_vm_types::storage::StorageGasParameters;
use move_vm_runtime::{config::VMConfig, RuntimeEnvironment, WithRuntimeEnvironment};
use sha3::{Digest, Sha3_256};
use std::sync::Arc;

/// A runtime environment which can be used for VM initialization and more. Contains features
/// used by execution, gas parameters, VM configs and global caches. Note that it is the user's
/// responsibility to make sure the environment is consistent, for now it should only be used per
/// block of transactions because all features or configs are updated only on per-block basis.
pub struct VelorEnvironment(Arc<Environment>);

impl VelorEnvironment {
    /// Returns new execution environment based on the current state.
    pub fn new(state_view: &impl StateView) -> Self {
        Self(Arc::new(Environment::new(state_view, false, None)))
    }

    /// Returns new execution environment based on the current state, also using the provided gas
    /// hook for native functions for gas calibration.
    pub fn new_with_gas_hook(
        state_view: &impl StateView,
        gas_hook: Arc<dyn Fn(DynamicExpression) + Send + Sync>,
    ) -> Self {
        Self(Arc::new(Environment::new(
            state_view,
            false,
            Some(gas_hook),
        )))
    }

    /// Returns new execution environment based on the current state, also injecting create signer
    /// native for government proposal simulation. Should not be used for regular execution.
    pub fn new_with_injected_create_signer_for_gov_sim(state_view: &impl StateView) -> Self {
        Self(Arc::new(Environment::new(state_view, true, None)))
    }

    /// Returns new environment but with delayed field optimization enabled. Should only be used by
    /// block executor where this optimization is needed. Note: whether the optimization will be
    /// enabled or not depends on the feature flag.
    pub fn new_with_delayed_field_optimization_enabled(state_view: &impl StateView) -> Self {
        let env = Environment::new(state_view, false, None).try_enable_delayed_field_optimization();
        Self(Arc::new(env))
    }

    /// Returns the [ChainId] used by this environment.
    #[inline]
    pub fn chain_id(&self) -> ChainId {
        self.0.chain_id
    }

    /// Returns the [Features] used by this environment.
    #[inline]
    pub fn features(&self) -> &Features {
        &self.0.features
    }

    /// Returns the [TimedFeatures] used by this environment.
    #[inline]
    pub fn timed_features(&self) -> &TimedFeatures {
        &self.0.timed_features
    }

    /// Returns the [VMConfig] used by this environment.
    #[inline]
    pub fn vm_config(&self) -> &VMConfig {
        self.0.runtime_environment.vm_config()
    }

    /// Returns the gas feature used by this environment.
    #[inline]
    pub fn gas_feature_version(&self) -> u64 {
        self.0.gas_feature_version
    }

    /// Returns the gas parameters used by this environment, and an error if they were not found
    /// on-chain.
    #[inline]
    pub fn gas_params(&self) -> &Result<VelorGasParameters, String> {
        &self.0.gas_params
    }

    /// Returns the storage gas parameters used by this environment, and an error if they were not
    /// found on-chain.
    #[inline]
    pub fn storage_gas_params(&self) -> &Result<StorageGasParameters, String> {
        &self.0.storage_gas_params
    }

    /// Returns true if create_signer native was injected for the government proposal simulation.
    /// Deprecated, and should not be used.
    #[inline]
    #[deprecated]
    pub fn inject_create_signer_for_gov_sim(&self) -> bool {
        #[allow(deprecated)]
        self.0.inject_create_signer_for_gov_sim
    }

    /// Returns bytes corresponding to the verifier config in this environment.
    pub fn verifier_config_bytes(&self) -> &Vec<u8> {
        &self.0.verifier_bytes
    }
}

impl Clone for VelorEnvironment {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl PartialEq for VelorEnvironment {
    fn eq(&self, other: &Self) -> bool {
        self.0.hash == other.0.hash
    }
}

impl Eq for VelorEnvironment {}

impl WithRuntimeEnvironment for VelorEnvironment {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        &self.0.runtime_environment
    }
}

struct Environment {
    /// Specifies the chain, i.e., testnet, mainnet, etc.
    chain_id: ChainId,

    /// Set of features enabled in this environment.
    features: Features,
    /// Set of timed features enabled in this environment.
    timed_features: TimedFeatures,

    /// Gas feature version used in this environment.
    gas_feature_version: u64,
    /// Gas parameters used in this environment. Error is stored if gas parameters were not found
    /// on-chain.
    gas_params: Result<VelorGasParameters, String>,
    /// Storage gas parameters used in this environment. Error is stored if gas parameters were not
    /// found on-chain.
    storage_gas_params: Result<StorageGasParameters, String>,

    /// The runtime environment, containing global struct type and name caches, and VM configs.
    runtime_environment: RuntimeEnvironment,

    /// True if we need to inject create signer native for government proposal simulation.
    /// Deprecated, and will be removed in the future.
    #[deprecated]
    inject_create_signer_for_gov_sim: bool,

    /// Hash of configs used in this environment. Used to be able to compare environments.
    hash: [u8; 32],
    /// Bytes of serialized verifier config. Used to detect any changes in verification configs.
    /// We stored bytes instead of hash because config is expected to be smaller than the crypto
    /// hash itself.
    verifier_bytes: Vec<u8>,
}

impl Environment {
    fn new(
        state_view: &impl StateView,
        inject_create_signer_for_gov_sim: bool,
        gas_hook: Option<Arc<dyn Fn(DynamicExpression) + Send + Sync>>,
    ) -> Self {
        // We compute and store a hash of configs in order to distinguish different environments.
        let mut sha3_256 = Sha3_256::new();
        let features =
            fetch_config_and_update_hash::<Features>(&mut sha3_256, state_view).unwrap_or_default();

        // If no chain ID is in storage, we assume we are in a testing environment.
        let chain_id = fetch_config_and_update_hash::<ChainId>(&mut sha3_256, state_view)
            .unwrap_or_else(ChainId::test);
        let timestamp_micros =
            fetch_config_and_update_hash::<ConfigurationResource>(&mut sha3_256, state_view)
                .map(|config| config.last_reconfiguration_time_micros())
                .unwrap_or(0);

        let mut timed_features_builder = TimedFeaturesBuilder::new(chain_id, timestamp_micros);
        if let Some(profile) = get_timed_feature_override() {
            // We need to ensure the override is taken into account for the hash.
            let profile_bytes = bcs::to_bytes(&profile)
                .expect("Timed features override should always be serializable");
            sha3_256.update(&profile_bytes);

            timed_features_builder = timed_features_builder.with_override_profile(profile)
        }
        let timed_features = timed_features_builder.build();

        // TODO(Gas):
        //   Right now, we have to use some dummy values for gas parameters if they are not found
        //   on-chain. This only happens in a edge case that is probably related to write set
        //   transactions or genesis, which logically speaking, shouldn't be handled by the VM at
        //   all. We should clean up the logic here once we get that refactored.
        let (gas_params, storage_gas_params, gas_feature_version) =
            get_gas_parameters(&mut sha3_256, &features, state_view);
        let (native_gas_params, misc_gas_params, ty_builder) = match &gas_params {
            Ok(gas_params) => {
                let ty_builder = velor_prod_ty_builder(gas_feature_version, gas_params);
                (
                    gas_params.natives.clone(),
                    gas_params.vm.misc.clone(),
                    ty_builder,
                )
            },
            Err(_) => {
                let ty_builder = velor_default_ty_builder();
                (
                    NativeGasParameters::zeros(),
                    MiscGasParameters::zeros(),
                    ty_builder,
                )
            },
        };

        let mut builder = SafeNativeBuilder::new(
            gas_feature_version,
            native_gas_params,
            misc_gas_params,
            timed_features.clone(),
            features.clone(),
            gas_hook,
        );
        let natives = velor_natives_with_builder(&mut builder, inject_create_signer_for_gov_sim);
        let vm_config =
            velor_prod_vm_config(gas_feature_version, &features, &timed_features, ty_builder);
        let verifier_bytes =
            bcs::to_bytes(&vm_config.verifier_config).expect("Verifier config is serializable");
        let runtime_environment = RuntimeEnvironment::new_with_config(natives, vm_config);

        let hash = sha3_256.finalize().into();

        #[allow(deprecated)]
        Self {
            chain_id,
            features,
            timed_features,
            gas_feature_version,
            gas_params,
            storage_gas_params,
            runtime_environment,
            inject_create_signer_for_gov_sim,
            hash,
            verifier_bytes,
        }
    }

    fn try_enable_delayed_field_optimization(mut self) -> Self {
        if self.features.is_aggregator_v2_delayed_fields_enabled() {
            self.runtime_environment.enable_delayed_field_optimization();
        }
        self
    }
}

/// Fetches config from storage and updates the hash if it exists. Returns the fetched config.
fn fetch_config_and_update_hash<T: OnChainConfig>(
    sha3_256: &mut Sha3_256,
    state_view: &impl StateView,
) -> Option<T> {
    let (config, bytes) = T::fetch_config_and_bytes(state_view)?;
    sha3_256.update(&bytes);
    Some(config)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use velor_types::{
        on_chain_config::{FeatureFlag, GasScheduleV2},
        state_store::{state_key::StateKey, state_value::StateValue, MockStateView},
    };
    use serde::Serialize;
    use std::collections::HashMap;

    #[test]
    fn test_new_environment() {
        // This creates an empty state.
        let state_view = MockStateView::empty();
        let env = Environment::new(&state_view, false, None);

        // Check default values.
        assert_eq!(&env.features, &Features::default());
        assert_eq!(env.chain_id.id(), ChainId::test().id());
        assert!(
            !env.runtime_environment
                .vm_config()
                .delayed_field_optimization_enabled
        );

        let env = env.try_enable_delayed_field_optimization();
        assert!(
            env.runtime_environment
                .vm_config()
                .delayed_field_optimization_enabled
        );
    }

    fn state_view_with_non_default_config<T: OnChainConfig + Serialize>(
        config: T,
    ) -> MockStateView<StateKey> {
        MockStateView::new(HashMap::from([(
            StateKey::resource(T::address(), &T::struct_tag()).unwrap(),
            StateValue::new_legacy(bcs::to_bytes(&config).unwrap().into()),
        )]))
    }

    #[test]
    fn test_environment_eq() {
        let state_view = MockStateView::empty();
        let environment_1 = VelorEnvironment::new(&state_view);
        let environment_2 = VelorEnvironment::new(&state_view);
        assert!(environment_1 == environment_2);
    }

    #[test]
    fn test_environment_ne() {
        let mut non_default_configuration = ConfigurationResource::default();
        assert_eq!(
            non_default_configuration.last_reconfiguration_time_micros(),
            0
        );
        non_default_configuration.set_last_reconfiguration_time_for_test(1);

        let mut non_default_features = Features::default();
        assert!(non_default_features.is_enabled(FeatureFlag::EMIT_FEE_STATEMENT));
        non_default_features.disable(FeatureFlag::EMIT_FEE_STATEMENT);

        let state_views = [
            MockStateView::empty(),
            // Change configuration resource (epoch change).
            state_view_with_non_default_config(non_default_configuration),
            // Change features set.
            state_view_with_non_default_config(non_default_features),
            // Different chain ID.
            state_view_with_non_default_config(ChainId::mainnet()),
            // Different gas schedules:
            //  - different feature version,
            //  - same feature version, but an extra parameter,
            //  - completely different gas schedule.
            state_view_with_non_default_config(GasScheduleV2 {
                feature_version: 12,
                entries: vec![],
            }),
            state_view_with_non_default_config(GasScheduleV2 {
                feature_version: 13,
                entries: vec![],
            }),
            state_view_with_non_default_config(GasScheduleV2 {
                feature_version: 12,
                entries: vec![(String::from("gas.param.base"), 12)],
            }),
            state_view_with_non_default_config(GasScheduleV2 {
                feature_version: 0,
                entries: vec![],
            }),
        ];
        for i in 0..state_views.len() {
            for j in 0..state_views.len() {
                if i != j {
                    let environment_1 = VelorEnvironment::new(&state_views[i]);
                    let environment_2 = VelorEnvironment::new(&state_views[j]);
                    assert!(environment_1 != environment_2);
                }
            }
        }
    }

    #[test]
    fn test_environment_with_injected_create_signer_for_gov_sim() {
        let state_view = MockStateView::empty();

        let not_injected_envs = [
            VelorEnvironment::new(&state_view),
            VelorEnvironment::new_with_gas_hook(&state_view, Arc::new(|_| {})),
            VelorEnvironment::new_with_delayed_field_optimization_enabled(&state_view),
        ];
        for env in not_injected_envs {
            #[allow(deprecated)]
            let not_enabled = !env.inject_create_signer_for_gov_sim();
            assert!(not_enabled);
        }

        // Injected.
        let env = VelorEnvironment::new_with_injected_create_signer_for_gov_sim(&state_view);
        #[allow(deprecated)]
        let enabled = env.inject_create_signer_for_gov_sim();
        assert!(enabled);
    }
}

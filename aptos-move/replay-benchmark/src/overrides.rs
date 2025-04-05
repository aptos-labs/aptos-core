// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Defines different overrides for on-chain state used for benchmarking. With overrides, past
//! transactions can be replayed on top of a modified state, and we can evaluate how it impacts
//! performance or other things. Supported overrides include:
//!   1. enabling feature flags,
//!   2. disabling feature flags,
//!   3. overriding gas feature version,
//!   4. changing modules (bytecode, metadata, etc.) and package information.

use anyhow::bail;
use aptos_framework::{natives::code::PackageRegistry, BuildOptions, BuiltPackage};
use aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION;
use aptos_logger::error;
use aptos_types::{
    on_chain_config::{FeatureFlag, Features, GasScheduleV2, OnChainConfig},
    state_store::{state_key::StateKey, state_value::StateValue, StateView},
};
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

/// Stores information about compiled Move packages and the build options used to create them. Used
/// by the override configuration to shadow existing on-chain modules with modules defined in these
/// packages.
struct PackageOverride {
    packages: Vec<BuiltPackage>,
    build_options: BuildOptions,
}

impl PackageOverride {
    /// Uses the provided build options to build multiple packages from the specified paths.
    fn new(package_paths: Vec<PathBuf>, build_options: BuildOptions) -> anyhow::Result<Self> {
        let packages = package_paths
            .into_iter()
            .map(|path| BuiltPackage::build(path, build_options.clone()))
            .collect::<anyhow::Result<_>>()?;
        Ok(Self {
            packages,
            build_options,
        })
    }
}

/// Stores all state overrides.
pub struct OverrideConfig {
    /// Feature flags to enable. Invariant: does not overlap with disabled features.
    additional_enabled_features: Vec<FeatureFlag>,
    /// Feature flags to disable. Invariant: does not overlap with enabled features.
    additional_disabled_features: Vec<FeatureFlag>,
    /// Gas feature version to use. Invariant: must be at most the latest version.
    gas_feature_version: Option<u64>,
    /// Information about overridden packages.
    package_override: PackageOverride,
}

impl OverrideConfig {
    pub fn new(
        additional_enabled_features: Vec<FeatureFlag>,
        additional_disabled_features: Vec<FeatureFlag>,
        gas_feature_version: Option<u64>,
        override_packages: Vec<PathBuf>,
        experimental_features: Vec<String>,
    ) -> anyhow::Result<Self> {
        let mut build_options = BuildOptions::move_2();
        build_options.experiments = experimental_features;
        let package_override = PackageOverride::new(override_packages, build_options)?;

        if !additional_enabled_features
            .iter()
            .all(|f| !additional_disabled_features.contains(f))
        {
            bail!("Enabled and disabled feature flags cannot overlap")
        }
        if matches!(gas_feature_version, Some(v) if v > LATEST_GAS_FEATURE_VERSION) {
            bail!(
                "Gas feature version must be at most the latest one: {}",
                LATEST_GAS_FEATURE_VERSION
            );
        }

        Ok(Self {
            additional_enabled_features,
            additional_disabled_features,
            gas_feature_version,
            package_override,
        })
    }

    pub(crate) fn get_state_override(
        &self,
        state_view: &impl StateView,
    ) -> HashMap<StateKey, StateValue> {
        let mut state_override = HashMap::new();

        // Enable/disable features.
        if !self.additional_enabled_features.is_empty()
            || !self.additional_disabled_features.is_empty()
        {
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
        }

        // Gas feature override.
        if let Some(gas_feature_version) = self.gas_feature_version {
            // Only support V2 gas schedule which has gas feature versions. Otherwise, V1 has 0
            // version at all times, and most likely it has been so long ago we will not replay
            // these transactions.
            let (gas_schedule_state_key, gas_schedule_state_value) =
                config_override::<GasScheduleV2, _>(state_view, |gas_schedule| {
                    gas_schedule.feature_version = gas_feature_version;
                });
            state_override.insert(gas_schedule_state_key, gas_schedule_state_value);
        }

        // Override packages.
        let mut overridden_package_registries = HashMap::new();
        for package in &self.package_override.packages {
            // Modify existing package metadata or add new one.
            let addresses = package
                .modules()
                .map(|m| *m.self_addr())
                .collect::<BTreeSet<_>>();
            assert_eq!(
                addresses.len(),
                1,
                "Modules in the same package must have the same address"
            );

            let package_address = addresses
                .last()
                .expect("Package must contain at least one module");
            let package_registry_state_key =
                StateKey::resource(package_address, &PackageRegistry::struct_tag())
                    .expect("Should always be able to create state key for package registry");

            let old_package_registry_state_value =
                match overridden_package_registries.remove(&package_registry_state_key) {
                    Some(state_value) => state_value,
                    None => state_view
                        .get_state_value(&package_registry_state_key)
                        .unwrap_or_else(|err| {
                            panic!(
                                "Failed to fetch package registry at {}: {:?}",
                                package_address, err
                            )
                        })
                        .expect("Package registry for override must always exist"),
                };

            let metadata = package.extract_metadata().unwrap_or_else(|err| {
                panic!(
                    "Failed to extract metadata for package {}: {:?}",
                    package.name(),
                    err
                )
            });
            let new_package_registry_state_value = old_package_registry_state_value
                .map_bytes(|bytes| {
                    let mut package_registry = bcs::from_bytes::<PackageRegistry>(&bytes)
                        .expect("Package registry should deserialize");

                    let mut metadata_idx = None;
                    for (idx, package_metadata) in package_registry.packages.iter().enumerate() {
                        if package_metadata.name == metadata.name {
                            metadata_idx = Some(idx);
                            break;
                        }
                    }
                    match metadata_idx {
                        Some(idx) => {
                            package_registry.packages[idx] = metadata;
                        },
                        None => {
                            package_registry.packages.push(metadata);
                        },
                    }

                    let bytes = bcs::to_bytes(&package_registry)
                        .expect("Package registry should serialize");
                    Ok(bytes.into())
                })
                .expect("Modifying package never returns an error");

            overridden_package_registries
                .insert(package_registry_state_key, new_package_registry_state_value);

            // Modify all existing modules or add new ones.
            let bytecode_version = self.package_override.build_options.bytecode_version;
            for module in package.modules() {
                let mut module_bytes = vec![];
                module
                    .serialize_for_version(bytecode_version, &mut module_bytes)
                    .unwrap_or_else(|err| {
                        panic!(
                            "Failed to serialize module {}::{}: {:?}",
                            module.self_addr(),
                            module.self_name(),
                            err
                        )
                    });

                let state_key = StateKey::module(module.self_addr(), module.self_name());
                let onchain_state_value =
                    state_view
                        .get_state_value(&state_key)
                        .unwrap_or_else(|err| {
                            panic!(
                                "Failed to fetch module {}::{}: {:?}",
                                module.self_addr(),
                                module.self_name(),
                                err
                            )
                        });
                let state_value = match onchain_state_value {
                    Some(state_value) => {
                        state_value.map_bytes(|_| Ok(module_bytes.into())).unwrap()
                    },

                    None => StateValue::new_legacy(module_bytes.into()),
                };
                if state_override.insert(state_key, state_value).is_some() {
                    panic!(
                        "Overriding module {}::{} more than once",
                        module.self_addr(),
                        module.self_name()
                    );
                }
            }
        }
        state_override.extend(overridden_package_registries);

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

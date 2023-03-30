// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod components;
mod utils;
pub mod validate;
pub use components::{ReleaseConfig, ReleaseEntry};
use once_cell::sync::Lazy;

const RELEASE_CONFIG: &str = include_str!("../data/release.yaml");

static CURRENT_RELEASE_CONFIG: Lazy<ReleaseConfig> =
    Lazy::new(|| ReleaseConfig::parse(RELEASE_CONFIG).expect("YAML NOT PARSABLE"));

/// Returns the release bundle with which the last testnet was build or updated.
pub fn current_release_config() -> &'static ReleaseConfig {
    &CURRENT_RELEASE_CONFIG
}

#[test]
// Check that the feature flags enabled at genesis matches with the release config file.
fn assert_feature_flags_eq() {
    use crate::components::feature_flags::FeatureFlag;
    use std::collections::HashSet;

    let config = current_release_config();
    let mut all_features = HashSet::new();
    for feature in &config.update_sequence {
        if let ReleaseEntry::FeatureFlag(features) = feature {
            for feature in &features.enabled {
                all_features.insert(feature.clone());
            }
        }
    }
    assert!(all_features.is_superset(
        &aptos_vm_genesis::default_features()
            .into_iter()
            .map(FeatureFlag::from)
            .collect::<HashSet<_>>()
    ));
}

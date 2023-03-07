// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod components;
mod utils;
pub mod validate;
pub use components::ReleaseConfig;
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

    let config = current_release_config();
    let features = aptos_vm_genesis::default_features();
    for feature in features {
        assert!(config
            .feature_flags
            .as_ref()
            .unwrap()
            .enabled
            .contains(&FeatureFlag::from(feature)));
    }
}

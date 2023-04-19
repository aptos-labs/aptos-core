// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod components;
mod utils;
pub mod validate;
pub use components::{ExecutionMode, ReleaseConfig, ReleaseEntry};
use once_cell::sync::Lazy;

// Update me after branch cut.
const RELEASE_CONFIG: &str = include_str!("../data/release.yaml");

static CURRENT_RELEASE_CONFIG: Lazy<ReleaseConfig> =
    Lazy::new(|| ReleaseConfig::parse(RELEASE_CONFIG).expect("YAML NOT PARSABLE"));

/// Returns the release bundle with which the last testnet was build or updated.
pub fn current_release_config() -> &'static ReleaseConfig {
    &CURRENT_RELEASE_CONFIG
}

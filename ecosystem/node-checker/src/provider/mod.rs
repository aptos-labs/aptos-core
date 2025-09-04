// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! todo explain that Providers should take in everything they need to operate
//! in their constructors.

pub mod api_index;
mod cache;
mod helpers;
pub mod metrics;
pub mod noise;
mod provider_collection;
pub mod system_information;
mod traits;

use self::{
    api_index::ApiIndexProviderConfig, metrics::MetricsProviderConfig, noise::NoiseProviderConfig,
    system_information::SystemInformationProviderConfig,
};
pub use helpers::MISSING_PROVIDER_MESSAGE;
pub use provider_collection::ProviderCollection;
use serde::{Deserialize, Serialize};
use std::time::Duration;
pub use traits::{Provider, ProviderError};

/// Some Providers might have configuration needs. They can put any configs here
/// and they'll get stored in the ProviderCollection so they can be used when
/// building new Providers.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfigs {
    pub api_index: ApiIndexProviderConfig,

    pub metrics: MetricsProviderConfig,

    pub system_information: SystemInformationProviderConfig,

    pub noise: NoiseProviderConfig,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct CommonProviderConfig {
    /// Some Checkers need to collect data twice during their run. This defines how
    /// many seconds those Checkers should sleep between each check.
    #[serde(default = "CommonProviderConfig::default_check_delay_secs")]
    pub check_delay_secs: u64,

    /// How long in milliseconds the Provider should cache its output before fetching
    /// the data again.
    #[serde(default = "CommonProviderConfig::default_cache_ttl_ms")]
    pub cache_ttl_ms: u64,
}

impl CommonProviderConfig {
    fn default_check_delay_secs() -> u64 {
        4
    }

    fn default_cache_ttl_ms() -> u64 {
        1000
    }

    pub fn check_delay(&self) -> Duration {
        Duration::from_secs(self.check_delay_secs)
    }
}

impl Default for CommonProviderConfig {
    fn default() -> Self {
        Self {
            check_delay_secs: Self::default_check_delay_secs(),
            cache_ttl_ms: Self::default_cache_ttl_ms(),
        }
    }
}

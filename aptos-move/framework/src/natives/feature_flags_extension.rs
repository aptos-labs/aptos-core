// Copyright © Aptos Foundation

use aptos_types::on_chain_config::Features;
use better_any::{Tid, TidAble};

/// Extension to store the feature flags inside a `NativeContext` so that a Rust native function can access them.
#[derive(Default, Tid)]
pub struct NativeFeatureFlagsExtension {
    features: Features,
}

impl NativeFeatureFlagsExtension {
    pub fn new(features: Features) -> Self {
        Self { features }
    }

    pub fn get_features(&self) -> &Features {
        &self.features
    }
}

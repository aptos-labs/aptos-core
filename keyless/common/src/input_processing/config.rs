// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct CircuitPaddingConfig {
    pub max_lengths: BTreeMap<String, usize>,
}

impl CircuitPaddingConfig {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            max_lengths: BTreeMap::new(),
        }
    }

    pub fn max_length(mut self, signal: &str, l: usize) -> Self {
        self.max_lengths.insert(String::from(signal), l);
        self
    }
}

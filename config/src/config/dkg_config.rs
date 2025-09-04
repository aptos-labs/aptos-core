// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DKGConfig {
    pub max_network_channel_size: usize,
}

impl Default for DKGConfig {
    fn default() -> Self {
        Self {
            max_network_channel_size: 256,
        }
    }
}

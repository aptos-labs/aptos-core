// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SfStreamerConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    false
}

impl Default for SfStreamerConfig {
    fn default() -> SfStreamerConfig {
        SfStreamerConfig {
            enabled: default_enabled(),
        }
    }
}

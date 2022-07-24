// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SfStreamerConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub target_address: SocketAddr,
}

fn default_enabled() -> bool {
    false
}

pub const DEFAULT_ADDRESS: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8083;

impl Default for SfStreamerConfig {
    fn default() -> SfStreamerConfig {
        SfStreamerConfig {
            enabled: default_enabled(),
            target_address: format!("{}:{}", DEFAULT_ADDRESS, DEFAULT_PORT)
                .parse()
                .unwrap(),
        }
    }
}

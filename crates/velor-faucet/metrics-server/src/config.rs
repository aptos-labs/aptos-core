// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MetricsServerConfig {
    /// Whether to disable the metrics server.
    #[serde(default = "MetricsServerConfig::default_disable")]
    pub disable: bool,

    /// What address to listen on, e.g. localhost / 0.0.0.0
    #[serde(default = "MetricsServerConfig::default_listen_address")]
    pub listen_address: String,

    /// What port to listen on.
    #[serde(default = "MetricsServerConfig::default_listen_port")]
    pub listen_port: u16,
}

impl MetricsServerConfig {
    fn default_disable() -> bool {
        false
    }

    fn default_listen_address() -> String {
        "0.0.0.0".to_string()
    }

    fn default_listen_port() -> u16 {
        9101
    }
}

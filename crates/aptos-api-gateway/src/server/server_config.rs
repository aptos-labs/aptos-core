// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    /// What address to listen on, e.g. localhost / 0.0.0.0
    #[serde(default = "default_listen_address")]
    pub listen_address: String,

    /// What port to listen on.
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,

    /// API path base. e.g. "/v1"
    #[serde(default = "default_api_path_base")]
    pub api_path_base: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_address: default_listen_address(),
            listen_port: default_listen_port(),
            api_path_base: default_api_path_base(),
        }
    }
}

fn default_listen_address() -> String {
    "0.0.0.0".to_string()
}

fn default_listen_port() -> u16 {
    10212
}

fn default_api_path_base() -> String {
    "".to_string()
}

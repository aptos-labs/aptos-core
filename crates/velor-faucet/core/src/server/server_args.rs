// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    /// What address to listen on, e.g. localhost / 0.0.0.0
    #[serde(default = "ServerConfig::default_listen_address")]
    pub listen_address: String,

    /// What port to listen on.
    #[serde(default = "ServerConfig::default_listen_port")]
    pub listen_port: u16,

    /// API path base. e.g. "/v1"
    #[serde(default = "ServerConfig::default_api_path_base")]
    pub api_path_base: String,
}

impl ServerConfig {
    fn default_listen_address() -> String {
        "0.0.0.0".to_string()
    }

    fn default_listen_port() -> u16 {
        10212
    }

    fn default_api_path_base() -> String {
        "".to_string()
    }
}

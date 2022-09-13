// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub address: SocketAddr,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_cert_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_key_path: Option<String>,
    // optional for compatible with old configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_length_limit: Option<u64>,
    #[serde(default = "default_disabled")]
    pub failpoints_enabled: bool,
    #[serde(default = "default_enabled")]
    pub json_output_enabled: bool,
    #[serde(default = "default_enabled")]
    pub bcs_output_enabled: bool,
    #[serde(default = "default_enabled")]
    pub encode_submission_enabled: bool,
    #[serde(default = "default_enabled")]
    pub transaction_submission_enabled: bool,
    #[serde(default = "default_enabled")]
    pub transaction_simulation_enabled: bool,

    pub max_submit_transaction_batch_size: usize,

    /// Maximum page size for paginated APIs
    pub max_transactions_page_size: u16,
    pub max_events_page_size: u16,
}

pub const DEFAULT_ADDRESS: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8080;
pub const DEFAULT_REQUEST_CONTENT_LENGTH_LIMIT: u64 = 8 * 1024 * 1024; // 8 MB
pub const DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE: usize = 100;
pub const DEFAULT_MAX_PAGE_SIZE: u16 = 1000;

fn default_enabled() -> bool {
    true
}

fn default_disabled() -> bool {
    false
}

impl Default for ApiConfig {
    fn default() -> ApiConfig {
        ApiConfig {
            enabled: default_enabled(),
            address: format!("{}:{}", DEFAULT_ADDRESS, DEFAULT_PORT)
                .parse()
                .unwrap(),
            tls_cert_path: None,
            tls_key_path: None,
            content_length_limit: None,
            failpoints_enabled: default_disabled(),
            bcs_output_enabled: default_enabled(),
            json_output_enabled: default_enabled(),
            encode_submission_enabled: default_enabled(),
            transaction_submission_enabled: default_enabled(),
            transaction_simulation_enabled: default_enabled(),
            max_submit_transaction_batch_size: DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE,
            max_transactions_page_size: DEFAULT_MAX_PAGE_SIZE,
            max_events_page_size: DEFAULT_MAX_PAGE_SIZE,
        }
    }
}

impl ApiConfig {
    pub fn randomize_ports(&mut self) {
        self.address.set_port(utils::get_available_port());
    }

    pub fn content_length_limit(&self) -> u64 {
        match self.content_length_limit {
            Some(v) => v,
            None => DEFAULT_REQUEST_CONTENT_LENGTH_LIMIT,
        }
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetbenchConfig {
    pub enabled: bool,
    pub max_network_channel_size: u64, // Max num of pending network messages
    pub netbench_service_threads: Option<usize>, // Number of kernel threads for tokio runtime. None default for num-cores.

    pub enable_direct_send_testing: bool, // Whether or not to enable direct send test mode
    pub direct_send_data_size: usize,     // The amount of data to send in each request
    pub direct_send_per_second: u64,      // The interval (microseconds) between requests

    pub enable_rpc_testing: bool,
    pub rpc_data_size: usize,
    pub rpc_per_second: u64,
    pub rpc_in_flight: usize,
}

impl Default for NetbenchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_network_channel_size: 1000,
            netbench_service_threads: Some(2),

            enable_direct_send_testing: false,
            direct_send_data_size: 100 * 1024, // 100 KB
            direct_send_per_second: 1_000,

            enable_rpc_testing: false,
            rpc_data_size: 100 * 1024,
            rpc_per_second: 1_000,
            rpc_in_flight: 8,
        }
    }
}

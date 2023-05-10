// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DagConfig {
    pub channel_size: usize,
    pub max_node_txns: u64,
    pub max_node_bytes: u64,

    pub state_sync_window: u64,
}

impl Default for DagConfig {
    fn default() -> DagConfig {
        DagConfig {
            channel_size: 100,
            // The best is probably to pull all local proofs
            max_node_txns: 1000,
            max_node_bytes: 8000000,

            state_sync_window: 60,
        }
    }
}

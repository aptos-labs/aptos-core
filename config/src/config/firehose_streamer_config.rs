// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct FirehoseStreamerConfig {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated = "please use starting_block instead"]
    pub starting_version: Option<u64>,
    // The block to start pushing out indexed data from, for the StreamingFast Firehose indexer
    // Alternatively can set the `STARTING_BLOCK` env var
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_block: Option<u64>,
}

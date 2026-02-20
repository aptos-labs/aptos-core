// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// This config is kept as a stub for backward compatibility with existing node
// config files. The legacy in-node indexer has been removed in favor of the
// separate transaction streaming service (indexer-grpc).

use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

#[derive(Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct IndexerConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postgres_uri: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_version: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_migrations: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_chain_id: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetch_tasks: Option<u8>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor_tasks: Option<u8>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emit_every: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_lookback_versions: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ans_contract_address: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nft_points_contract: Option<String>,
}

impl Debug for IndexerConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IndexerConfig {{ deprecated }}")
    }
}

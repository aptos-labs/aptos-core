// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{on_chain_config::OnChainConfig, state_store::NUM_STATE_SHARDS};
use anyhow::{format_err, Result};
use serde::{Deserialize, Serialize};

/// The on-chain hot state config, wrapped in an enum to allow adding fields later.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainHotStateConfig {
    V1(HotStateConfigV1),
}

/// Matches the currently hardcoded behavior.
impl Default for OnChainHotStateConfig {
    fn default() -> Self {
        Self::V1(HotStateConfigV1 {
            max_items_per_shard: vec![250_000; NUM_STATE_SHARDS],
            max_value_bytes_per_shard: vec![u64::MAX; NUM_STATE_SHARDS],
            refresh_interval_versions: 100_000,
            max_promotions_per_block: 10_240,
        })
    }
}

impl OnChainConfig for OnChainHotStateConfig {
    const MODULE_IDENTIFIER: &'static str = "hot_state_config";
    const TYPE_IDENTIFIER: &'static str = "HotStateConfig";

    /// The Move resource is
    /// ```ignore
    /// struct HotStateConfig has drop, key, store {
    ///    config: vector<u8>,
    /// }
    /// ```
    /// so we need two rounds of bcs deserialization to turn it back to OnChainHotStateConfig
    fn deserialize_into_config(bytes: &[u8]) -> Result<Self> {
        let raw_bytes: Vec<u8> = bcs::from_bytes(bytes)?;
        bcs::from_bytes(&raw_bytes)
            .map_err(|e| format_err!("[on-chain config] Failed to deserialize into config: {}", e))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HotStateConfigV1 {
    /// Max number of items in each shard, indexed by shard id.
    pub max_items_per_shard: Vec<u64>,
    /// Max sum of value sizes in each shard, indexed by shard id. Values vary a lot in size, so
    /// capping the item count alone doesn't bound memory usage.
    pub max_value_bytes_per_shard: Vec<u64>,
    /// Refresh `hot_since_version` of a hot item when it's accessed again after this many
    /// versions, to keep frequently read items from being evicted.
    pub refresh_interval_versions: u64,
    /// Max number of read-only keys promoted by the block epilogue, to keep it from being too
    /// heavy.
    pub max_promotions_per_block: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::on_chain_config::{InMemoryOnChainConfig, OnChainConfigPayload};
    use std::collections::HashMap;

    #[test]
    fn test_config_onchain_payload() {
        let config = OnChainHotStateConfig::default();

        let mut configs = HashMap::new();
        configs.insert(
            OnChainHotStateConfig::CONFIG_ID,
            // Requires double serialization, check deserialize_into_config for more details
            bcs::to_bytes(&bcs::to_bytes(&config).unwrap()).unwrap(),
        );
        let payload = OnChainConfigPayload::new(1, InMemoryOnChainConfig::new(configs));

        let result: OnChainHotStateConfig = payload.get().unwrap();
        assert_eq!(result, config);
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{on_chain_config::OnChainConfig, state_store::NUM_STATE_SHARDS};
use anyhow::{ensure, format_err, Result};
use serde::{Deserialize, Serialize};

/// The on-chain hot state config, wrapped in an enum to allow adding fields later.
///
/// A new config takes effect in the reconfig txn that applies it: the hot state is evicted down to
/// the new `max_items_per_shard` and `max_bytes_per_shard` at that txn, while the other fields apply
/// from the next epoch.
///
/// N.b. lowering either cap evicts all the excess items in the reconfig txn at once, which can make
/// that block heavy. Consider lowering them in steps.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainHotStateConfig {
    V1(HotStateConfigV1),
}

impl OnChainHotStateConfig {
    /// Backs `0x1::hot_state_config::set_for_next_epoch`, so nodes never apply an invalid config.
    /// Changing the rules changes execution results, and tightening them makes nodes panic on a
    /// config already on chain.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::V1(config) => {
                ensure!(
                    config.max_items_per_shard.len() == NUM_STATE_SHARDS,
                    "Expecting {} entries in max_items_per_shard, got {}.",
                    NUM_STATE_SHARDS,
                    config.max_items_per_shard.len(),
                );
                ensure!(
                    config.max_bytes_per_shard.len() == NUM_STATE_SHARDS,
                    "Expecting {} entries in max_bytes_per_shard, got {}.",
                    NUM_STATE_SHARDS,
                    config.max_bytes_per_shard.len(),
                );
                ensure!(
                    config.max_items_per_shard.iter().all(|n| *n > 0),
                    "max_items_per_shard must be positive: {:?}",
                    config.max_items_per_shard,
                );
            },
        }
        Ok(())
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
    /// Max sum of key and value sizes in each shard, indexed by shard id. Each key counts as its
    /// 32-byte hash, which is all the hot state keeps. Values vary a lot in size, so capping the
    /// item count alone doesn't bound memory usage.
    pub max_bytes_per_shard: Vec<u64>,
    /// Refresh `hot_since_version` of a hot item when it's accessed again after this many
    /// versions, to keep frequently read items from being evicted.
    pub refresh_interval_versions: u64,
    /// Max total size of the read-only keys promoted by the block epilogue, as measured by
    /// `StateKey::size`. The epilogue txn carries each of them in both its payload and its write
    /// set, and a key can be large, so capping the count alone doesn't bound the txn size.
    pub max_promotion_bytes_per_block: u64,
    /// If true, writes to hot items reach the cold state only when the items are evicted, so the
    /// cold state stops paying for hot writes. Otherwise every write goes to both.
    // TODO(HotState): not honored yet.
    pub defer_cold_writes_to_eviction: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::on_chain_config::{InMemoryOnChainConfig, OnChainConfigPayload};
    use std::collections::HashMap;

    #[test]
    fn test_config_onchain_payload() {
        let config = OnChainHotStateConfig::V1(HotStateConfigV1 {
            max_items_per_shard: vec![250_000; NUM_STATE_SHARDS],
            max_bytes_per_shard: vec![1 << 30; NUM_STATE_SHARDS],
            refresh_interval_versions: 100_000,
            max_promotion_bytes_per_block: 512 * 1024,
            defer_cold_writes_to_eviction: false,
        });

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

    #[test]
    fn test_validate() {
        let config = |max_items_per_shard, max_bytes_per_shard| {
            OnChainHotStateConfig::V1(HotStateConfigV1 {
                max_items_per_shard,
                max_bytes_per_shard,
                refresh_interval_versions: 0,
                max_promotion_bytes_per_block: 0,
                defer_cold_writes_to_eviction: false,
            })
        };
        let valid = || vec![1; NUM_STATE_SHARDS];

        config(valid(), valid()).validate().unwrap();
        // A zero byte cap is valid, if useless.
        config(valid(), vec![0; NUM_STATE_SHARDS])
            .validate()
            .unwrap();
        for wrong_len in [NUM_STATE_SHARDS - 1, NUM_STATE_SHARDS + 1] {
            config(vec![1; wrong_len], valid()).validate().unwrap_err();
            config(valid(), vec![1; wrong_len]).validate().unwrap_err();
        }
        let mut with_zero = valid();
        with_zero[3] = 0;
        config(with_zero, valid()).validate().unwrap_err();
    }
}

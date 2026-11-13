// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::DbReader;
use anyhow::Result;
use aptos_config::config::HotStateConfig;
use aptos_types::{
    on_chain_config::{OnChainConfig, OnChainHotStateConfig},
    state_store::{state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
};
use once_cell::sync::Lazy;
use std::num::NonZeroUsize;

pub(crate) static ON_CHAIN_CONFIG_KEY: Lazy<StateKey> = Lazy::new(|| {
    StateKey::on_chain_config::<OnChainHotStateConfig>().expect("Must be a valid state key.")
});

/// The hot state config in effect at some version: the on-chain config if it exists, otherwise the
/// local config.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectiveHotStateConfig {
    /// The fallback, kept to resolve later changes of the on-chain config.
    local: HotStateConfig,
    max_items_per_shard: [NonZeroUsize; NUM_STATE_SHARDS],
    max_bytes_per_shard: [usize; NUM_STATE_SHARDS],
    refresh_interval_versions: Version,
}

impl EffectiveHotStateConfig {
    pub fn from_local(local: HotStateConfig) -> Self {
        let max_items = NonZeroUsize::new(local.max_items_per_shard)
            .expect("max_items_per_shard must be positive.");
        Self {
            local,
            max_items_per_shard: [max_items; NUM_STATE_SHARDS],
            max_bytes_per_shard: [local.max_bytes_per_shard; NUM_STATE_SHARDS],
            refresh_interval_versions: local.refresh_interval_versions,
        }
    }

    /// `on_chain` is the raw on-chain config resource. `local` applies only if it doesn't exist.
    pub fn new(local: HotStateConfig, on_chain: Option<&[u8]>) -> Self {
        let Some(bytes) = on_chain else {
            return Self::from_local(local);
        };
        let config = Self::parse_on_chain(bytes)
            .expect("hot_state_config::set_for_next_epoch only accepts valid configs.");
        match config {
            OnChainHotStateConfig::V1(config) => Self {
                local,
                max_items_per_shard: std::array::from_fn(|shard_id| {
                    NonZeroUsize::new(config.max_items_per_shard[shard_id] as usize)
                        .expect("Already validated.")
                }),
                max_bytes_per_shard: std::array::from_fn(|shard_id| {
                    config.max_bytes_per_shard[shard_id] as usize
                }),
                refresh_interval_versions: config.refresh_interval_versions,
            },
        }
    }

    /// Reads the config in effect at `version`, and whether the on-chain config exists.
    pub fn read_at(
        local: HotStateConfig,
        reader: &dyn DbReader,
        version: Option<Version>,
    ) -> Result<(Self, bool)> {
        let on_chain = match version {
            Some(version) => reader.get_state_value_by_version(&ON_CHAIN_CONFIG_KEY, version)?,
            None => None,
        };
        let config = Self::new(local, on_chain.as_ref().map(|value| value.bytes().as_ref()));
        Ok((config, on_chain.is_some()))
    }

    /// Resolves a new value of the on-chain config, keeping the same fallback.
    pub(crate) fn with_on_chain(&self, on_chain: Option<&[u8]>) -> Self {
        Self::new(self.local, on_chain)
    }

    fn parse_on_chain(bytes: &[u8]) -> Result<OnChainHotStateConfig> {
        let config = OnChainHotStateConfig::deserialize_into_config(bytes)?;
        config.validate()?;
        Ok(config)
    }

    pub fn max_items(&self, shard_id: usize) -> NonZeroUsize {
        self.max_items_per_shard[shard_id]
    }

    pub fn max_bytes(&self, shard_id: usize) -> usize {
        self.max_bytes_per_shard[shard_id]
    }

    pub fn refresh_interval_versions(&self) -> Version {
        self.refresh_interval_versions
    }
}

/// Walks through the config changes in a batch of updates, in version order. A change made at a
/// version applies to the eviction at that version (the reconfig txn is a checkpoint), and to
/// updates after it.
pub(crate) struct ConfigCursor<'a> {
    current: EffectiveHotStateConfig,
    pending: &'a [(Version, EffectiveHotStateConfig)],
}

impl<'a> ConfigCursor<'a> {
    pub fn new(
        current: EffectiveHotStateConfig,
        changes: &'a [(Version, EffectiveHotStateConfig)],
    ) -> Self {
        Self {
            current,
            pending: changes,
        }
    }

    pub fn current(&self) -> &EffectiveHotStateConfig {
        &self.current
    }

    pub fn for_update(&mut self, version: Version) -> &EffectiveHotStateConfig {
        self.advance(|changed_at| changed_at < version)
    }

    pub fn for_eviction(&mut self, checkpoint_version: Version) -> &EffectiveHotStateConfig {
        self.advance(|changed_at| changed_at <= checkpoint_version)
    }

    fn advance(&mut self, applies: impl Fn(Version) -> bool) -> &EffectiveHotStateConfig {
        while let Some(((changed_at, config), rest)) = self.pending.split_first()
            && applies(*changed_at)
        {
            self.current = *config;
            self.pending = rest;
        }
        &self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::on_chain_config::HotStateConfigV1;

    const LOCAL: HotStateConfig = HotStateConfig {
        max_items_per_shard: 100,
        max_bytes_per_shard: 10_000,
        refresh_interval_versions: 10,
        delete_on_restart: false,
    };

    fn on_chain_bytes(
        max_items_per_shard: Vec<u64>,
        max_bytes_per_shard: Vec<u64>,
        refresh_interval_versions: u64,
    ) -> Vec<u8> {
        let config = OnChainHotStateConfig::V1(HotStateConfigV1 {
            max_items_per_shard,
            max_bytes_per_shard,
            refresh_interval_versions,
            max_promotion_bytes_per_block: 0,
            defer_cold_writes_to_eviction: false,
        });
        bcs::to_bytes(&bcs::to_bytes(&config).unwrap()).unwrap()
    }

    #[test]
    fn test_missing_on_chain_config_uses_local() {
        let config = EffectiveHotStateConfig::new(LOCAL, None);
        assert_eq!(config, EffectiveHotStateConfig::from_local(LOCAL));
        assert_eq!(config.max_items(0).get(), 100);
        assert_eq!(config.max_bytes(0), 10_000);
        assert_eq!(config.refresh_interval_versions(), 10);
    }

    #[test]
    fn test_on_chain_config_overrides_local() {
        let max_items = (1..=NUM_STATE_SHARDS as u64).collect();
        let max_bytes = (0..NUM_STATE_SHARDS as u64).map(|i| i * 1000).collect();
        let config =
            EffectiveHotStateConfig::new(LOCAL, Some(&on_chain_bytes(max_items, max_bytes, 7)));
        for shard_id in 0..NUM_STATE_SHARDS {
            assert_eq!(config.max_items(shard_id).get(), shard_id + 1);
            assert_eq!(config.max_bytes(shard_id), shard_id * 1000);
        }
        assert_eq!(config.refresh_interval_versions(), 7);

        // Deleting the on-chain config falls back to local.
        assert_eq!(
            config.with_on_chain(None),
            EffectiveHotStateConfig::from_local(LOCAL)
        );
    }

    #[test]
    #[should_panic(expected = "only accepts valid configs")]
    fn test_invalid_on_chain_config_panics() {
        let zero_items = on_chain_bytes(vec![0; NUM_STATE_SHARDS], vec![5; NUM_STATE_SHARDS], 7);
        EffectiveHotStateConfig::new(LOCAL, Some(&zero_items));
    }
}

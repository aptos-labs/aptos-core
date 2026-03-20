// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Allowlist cache for custom contract authentication.
//!
//! Uses a background refresh pattern (like validator_cache) to keep allowlists fresh.
//! A single background task periodically fetches allowlists for all configured contracts,
//! eliminating thundering herd issues and ensuring low-latency cache lookups.

use crate::{
    custom_contract_auth::{fetch_addresses_via_resource, fetch_addresses_via_view_function},
    debug, error,
    metrics::{
        AllowlistCacheOp, ALLOWLIST_CACHE_ENTRIES, ALLOWLIST_CACHE_LAST_UPDATE_TIMESTAMP,
        ALLOWLIST_CACHE_OPERATIONS, ALLOWLIST_CACHE_SIZE, ALLOWLIST_CACHE_UPDATE_FAILED_COUNT,
        ALLOWLIST_CACHE_UPDATE_SUCCESS_COUNT,
    },
    CustomContractConfig, OnChainAuthMethod,
};
use aptos_infallible::RwLock;
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time;

/// Allowlist data stored in the cache
#[derive(Clone, Debug)]
struct CachedAllowlist {
    addresses: HashSet<AccountAddress>,
}

impl CachedAllowlist {
    fn new(addresses: HashSet<AccountAddress>) -> Self {
        Self { addresses }
    }

    fn contains(&self, address: &AccountAddress) -> bool {
        self.addresses.contains(address)
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct CacheKey {
    contract_name: String,
    chain_id: ChainId,
}

/// Simple cache for allowlist data. Updated by AllowlistCacheUpdater.
#[derive(Clone, Default)]
pub struct AllowlistCache {
    cache: Arc<RwLock<HashMap<CacheKey, CachedAllowlist>>>,
}

impl AllowlistCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if an address is in the cached allowlist.
    /// Returns `Some(true)` if address is allowed, `Some(false)` if not allowed,
    /// or `None` if no cache entry exists.
    pub fn check_address(
        &self,
        contract_name: &str,
        chain_id: &ChainId,
        address: &AccountAddress,
    ) -> Option<bool> {
        let key = CacheKey {
            contract_name: contract_name.to_string(),
            chain_id: *chain_id,
        };

        let chain_id_str = chain_id.to_string();
        let cache = self.cache.read();
        if let Some(cached) = cache.get(&key) {
            let is_allowed = cached.contains(address);
            ALLOWLIST_CACHE_OPERATIONS
                .with_label_values(&[contract_name, &chain_id_str, AllowlistCacheOp::Hit.as_str()])
                .inc();
            Some(is_allowed)
        } else {
            ALLOWLIST_CACHE_OPERATIONS
                .with_label_values(&[
                    contract_name,
                    &chain_id_str,
                    AllowlistCacheOp::Miss.as_str(),
                ])
                .inc();
            None
        }
    }

    /// Update the cache with a new allowlist (called by AllowlistCacheUpdater)
    pub fn update(&self, contract_name: &str, chain_id: &ChainId, addresses: Vec<AccountAddress>) {
        let key = CacheKey {
            contract_name: contract_name.to_string(),
            chain_id: *chain_id,
        };

        let address_count = addresses.len();
        let cached = CachedAllowlist::new(addresses.into_iter().collect());

        let chain_id_str = chain_id.to_string();
        let mut cache = self.cache.write();
        cache.insert(key, cached);

        // Record update operation and update metrics
        ALLOWLIST_CACHE_OPERATIONS
            .with_label_values(&[
                contract_name,
                &chain_id_str,
                AllowlistCacheOp::Update.as_str(),
            ])
            .inc();

        // Update timestamp for this contract/chain (unix seconds for staleness alerting)
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        ALLOWLIST_CACHE_LAST_UPDATE_TIMESTAMP
            .with_label_values(&[contract_name, &chain_id_str])
            .set(now_unix as i64);

        // Update size metrics
        ALLOWLIST_CACHE_SIZE
            .with_label_values(&[contract_name, &chain_id_str])
            .set(address_count as i64);
        ALLOWLIST_CACHE_ENTRIES.set(cache.len() as i64);
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        CacheStats {
            total_entries: cache.len(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CacheStats {
    pub total_entries: usize,
}

/// Background updater for allowlist cache (similar to PeerSetCacheUpdater).
/// Periodically fetches allowlists for all configured custom contracts.
#[derive(Clone)]
pub struct AllowlistCacheUpdater {
    cache: AllowlistCache,
    contracts: Arc<Vec<CustomContractConfig>>,
    update_interval: Duration,
}

impl AllowlistCacheUpdater {
    pub fn new(
        cache: AllowlistCache,
        contracts: Vec<CustomContractConfig>,
        update_interval: Duration,
    ) -> Self {
        Self {
            cache,
            contracts: Arc::new(contracts),
            update_interval,
        }
    }

    /// Start the background update loop.
    /// Spawns a tokio task that runs until the process exits.
    pub fn run(self) {
        let mut interval = time::interval(self.update_interval);
        tokio::spawn(async move {
            // Do initial update immediately
            self.update().await;
            loop {
                interval.tick().await;
                self.update().await;
            }
        });
    }

    /// Update allowlists for all configured contracts
    async fn update(&self) {
        for contract in self.contracts.iter() {
            match self.update_contract(contract).await {
                Ok(Some(count)) => {
                    ALLOWLIST_CACHE_UPDATE_SUCCESS_COUNT
                        .with_label_values(&[&contract.name])
                        .inc();
                    debug!(
                        "allowlist cache update successful for contract '{}': {} addresses",
                        contract.name, count
                    );
                },
                Ok(None) => {
                    // Contract has no on_chain_auth (open telemetry mode) - nothing to update
                },
                Err(err) => {
                    // Log error but don't clear cache - stale data is better than no data
                    // The ALLOWLIST_CACHE_LAST_UPDATE_TIMESTAMP metric will show staleness
                    // Operators should alert on: now() - last_update_timestamp > threshold
                    ALLOWLIST_CACHE_UPDATE_FAILED_COUNT
                        .with_label_values(&[&contract.name, err.error_type()])
                        .inc();
                    error!(
                        "allowlist cache update failed for contract '{}': {:?} (using stale cache)",
                        contract.name, err
                    );
                },
            }
        }
    }

    /// Update allowlist for a single contract.
    /// Returns Ok(None) if the contract has no on_chain_auth (open telemetry mode).
    async fn update_contract(
        &self,
        contract: &CustomContractConfig,
    ) -> Result<Option<usize>, AllowlistUpdateError> {
        // Skip contracts without on_chain_auth (open telemetry mode)
        let config = match &contract.on_chain_auth {
            Some(config) => config,
            None => {
                debug!(
                    "skipping allowlist update for contract '{}' (no on_chain_auth configured)",
                    contract.name
                );
                return Ok(None);
            },
        };
        let chain_id = ChainId::new(config.chain_id);

        // Resolve the resource/function path (with env var substitution)
        let path = config
            .resolve_resource_path()
            .map_err(AllowlistUpdateError::ConfigError)?;

        // Resolve view function arguments (with env var substitution)
        let view_args = config
            .resolve_view_function_args()
            .map_err(AllowlistUpdateError::ConfigError)?;

        // Get REST URL
        let rest_url = config.rest_api_url.clone().ok_or_else(|| {
            AllowlistUpdateError::ConfigError("No REST URL configured".to_string())
        })?;

        // Fetch address list based on method
        let addresses = match config.method {
            OnChainAuthMethod::ViewFunction => fetch_addresses_via_view_function(
                &rest_url,
                &path,
                &view_args,
                &config.address_list_field,
            )
            .await
            .map_err(|e| AllowlistUpdateError::FetchError(e.to_string()))?,
            OnChainAuthMethod::Resource => {
                fetch_addresses_via_resource(&rest_url, &path, &config.address_list_field)
                    .await
                    .map_err(|e| AllowlistUpdateError::FetchError(e.to_string()))?
            },
        };

        let count = addresses.len();
        self.cache.update(&contract.name, &chain_id, addresses);
        Ok(Some(count))
    }
}

/// Errors that can occur during allowlist update
#[derive(Debug)]
pub enum AllowlistUpdateError {
    ConfigError(String),
    FetchError(String),
}

impl AllowlistUpdateError {
    /// Get error type as string for metrics labels
    pub fn error_type(&self) -> &'static str {
        match self {
            AllowlistUpdateError::ConfigError(_) => "config_error",
            AllowlistUpdateError::FetchError(_) => "fetch_error",
        }
    }
}

impl std::fmt::Display for AllowlistUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AllowlistUpdateError::ConfigError(s) => write!(f, "config error: {}", s),
            AllowlistUpdateError::FetchError(s) => write!(f, "fetch error: {}", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache = AllowlistCache::new();
        let contract_name = "test_contract";
        let chain_id = ChainId::new(1);
        let addr1 = AccountAddress::from_hex_literal("0x1").unwrap();
        let addr2 = AccountAddress::from_hex_literal("0x2").unwrap();

        // Initially not cached
        assert_eq!(cache.check_address(contract_name, &chain_id, &addr1), None);

        // Update cache
        cache.update(contract_name, &chain_id, vec![addr1]);

        // Should be cached and found
        assert_eq!(
            cache.check_address(contract_name, &chain_id, &addr1),
            Some(true)
        );

        // Different address should not be found
        assert_eq!(
            cache.check_address(contract_name, &chain_id, &addr2),
            Some(false)
        );
    }

    #[test]
    fn test_cache_per_contract_and_chain() {
        let cache = AllowlistCache::new();
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();

        // Different contracts
        cache.update("contract1", &ChainId::new(1), vec![addr]);
        cache.update("contract2", &ChainId::new(1), vec![]);

        assert_eq!(
            cache.check_address("contract1", &ChainId::new(1), &addr),
            Some(true)
        );
        assert_eq!(
            cache.check_address("contract2", &ChainId::new(1), &addr),
            Some(false)
        );

        // Different chains
        cache.update("contract1", &ChainId::new(2), vec![]);

        assert_eq!(
            cache.check_address("contract1", &ChainId::new(1), &addr),
            Some(true)
        );
        assert_eq!(
            cache.check_address("contract1", &ChainId::new(2), &addr),
            Some(false)
        );
    }
}

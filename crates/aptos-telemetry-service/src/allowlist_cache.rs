// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Allowlist cache to reduce blockchain queries during authentication.

use aptos_infallible::RwLock;
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

const ALLOWLIST_CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Clone, Debug)]
struct CachedAllowlist {
    addresses: HashSet<AccountAddress>,
    expires_at: Instant,
}

impl CachedAllowlist {
    fn new(addresses: HashSet<AccountAddress>, ttl: Duration) -> Self {
        Self {
            addresses,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct CacheKey {
    contract_name: String,
    chain_id: ChainId,
}

#[derive(Clone)]
pub struct AllowlistCache {
    cache: Arc<RwLock<HashMap<CacheKey, CachedAllowlist>>>,
    ttl: Duration,
}

impl AllowlistCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: ALLOWLIST_CACHE_TTL,
        }
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

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

        let cache = self.cache.read();
        if let Some(cached) = cache.get(&key) {
            if !cached.is_expired() {
                return Some(cached.addresses.contains(address));
            }
        }
        None
    }

    /// Update the cache with a new allowlist
    pub fn update(&self, contract_name: &str, chain_id: &ChainId, addresses: Vec<AccountAddress>) {
        let key = CacheKey {
            contract_name: contract_name.to_string(),
            chain_id: *chain_id,
        };

        let cached = CachedAllowlist::new(addresses.into_iter().collect(), self.ttl);

        let mut cache = self.cache.write();
        cache.insert(key, cached);
    }

    /// Clear expired entries from the cache
    #[allow(dead_code)]
    pub fn cleanup_expired(&self) {
        let mut cache = self.cache.write();
        cache.retain(|_, v| !v.is_expired());
    }

    /// Clear all entries for a specific contract
    #[allow(dead_code)]
    pub fn clear_contract(&self, contract_name: &str) {
        let mut cache = self.cache.write();
        cache.retain(|k, _| k.contract_name != contract_name);
    }

    /// Clear the entire cache
    #[allow(dead_code)]
    pub fn clear_all(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let total_entries = cache.len();
        let expired_entries = cache.values().filter(|v| v.is_expired()).count();

        CacheStats {
            total_entries,
            active_entries: total_entries - expired_entries,
            expired_entries,
        }
    }
}

impl Default for AllowlistCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CacheStats {
    pub total_entries: usize,
    pub active_entries: usize,
    pub expired_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

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
    fn test_cache_expiration() {
        let cache = AllowlistCache::with_ttl(Duration::from_millis(100));
        let contract_name = "test_contract";
        let chain_id = ChainId::new(1);
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();

        cache.update(contract_name, &chain_id, vec![addr]);

        // Should be cached immediately
        assert_eq!(
            cache.check_address(contract_name, &chain_id, &addr),
            Some(true)
        );

        // Wait for expiration
        sleep(Duration::from_millis(150));

        // Should be expired
        assert_eq!(cache.check_address(contract_name, &chain_id, &addr), None);
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = AllowlistCache::with_ttl(Duration::from_millis(50));
        let contract_name = "test_contract";
        let chain_id = ChainId::new(1);
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();

        cache.update(contract_name, &chain_id, vec![addr]);

        let stats_before = cache.stats();
        assert_eq!(stats_before.total_entries, 1);

        // Wait for expiration
        sleep(Duration::from_millis(100));

        // Cleanup expired entries
        cache.cleanup_expired();

        let stats_after = cache.stats();
        assert_eq!(stats_after.total_entries, 0);
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

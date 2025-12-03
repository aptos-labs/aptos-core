// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Challenge cache for secure challenge-response authentication.
//!
//! This cache stores server-issued challenges to prevent:
//! - Replay attacks (same challenge cannot be used twice)
//! - Bypass attacks (clients cannot use self-generated challenges)

use crate::metrics::{
    ChallengeCacheOp, CHALLENGE_CACHE_KEYS, CHALLENGE_CACHE_LAST_STORE_TIMESTAMP,
    CHALLENGE_CACHE_OPERATIONS, CHALLENGE_CACHE_SIZE,
};
use aptos_infallible::RwLock;
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Default TTL for challenges (5 minutes)
const CHALLENGE_CACHE_TTL: Duration = Duration::from_secs(300);

/// Maximum number of pending challenges per address to prevent memory exhaustion
const MAX_CHALLENGES_PER_ADDRESS: usize = 5;

/// A cached challenge with expiration metadata
#[derive(Clone, Debug)]
struct CachedChallenge {
    /// The challenge string
    challenge: String,
    /// When this challenge expires (absolute time)
    expires_at: Instant,
}

impl CachedChallenge {
    fn new(challenge: String, ttl: Duration) -> Self {
        Self {
            challenge,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Key for the challenge cache: (contract_name, chain_id, address)
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct CacheKey {
    contract_name: String,
    chain_id: ChainId,
    address: AccountAddress,
}

/// Thread-safe cache for server-issued challenges
#[derive(Clone)]
pub struct ChallengeCache {
    /// Map from cache key to list of pending challenges for that key
    cache: Arc<RwLock<HashMap<CacheKey, Vec<CachedChallenge>>>>,
    /// TTL for challenges
    ttl: Duration,
}

impl ChallengeCache {
    /// Create a new challenge cache with default TTL
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: CHALLENGE_CACHE_TTL,
        }
    }

    /// Create a new challenge cache with custom TTL
    #[cfg(test)]
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Store a newly issued challenge
    ///
    /// Returns the expiration timestamp (unix seconds) for the challenge
    pub fn store_challenge(
        &self,
        contract_name: &str,
        chain_id: &ChainId,
        address: &AccountAddress,
        challenge: String,
    ) -> u64 {
        let key = CacheKey {
            contract_name: contract_name.to_string(),
            chain_id: *chain_id,
            address: *address,
        };

        let cached = CachedChallenge::new(challenge, self.ttl);
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at_unix = now_unix + self.ttl.as_secs();

        let mut cache = self.cache.write();
        let challenges = cache.entry(key).or_default();

        // Remove expired challenges
        challenges.retain(|c| !c.is_expired());

        // Limit the number of pending challenges per address
        let evicted = challenges.len() >= MAX_CHALLENGES_PER_ADDRESS;
        if evicted {
            // Remove the oldest challenge
            challenges.remove(0);
            CHALLENGE_CACHE_OPERATIONS
                .with_label_values(&[contract_name, ChallengeCacheOp::Evicted.as_str()])
                .inc();
        }

        challenges.push(cached);

        // Update metrics: record store operation and update gauges
        CHALLENGE_CACHE_OPERATIONS
            .with_label_values(&[contract_name, ChallengeCacheOp::Store.as_str()])
            .inc();
        CHALLENGE_CACHE_LAST_STORE_TIMESTAMP
            .with_label_values(&[contract_name])
            .set(now_unix as i64);

        // Update size metrics per contract (count challenges for this contract only)
        let contract_challenges: usize = cache
            .iter()
            .filter(|(k, _)| k.contract_name == contract_name)
            .map(|(_, v)| v.len())
            .sum();
        let contract_keys: usize = cache
            .iter()
            .filter(|(k, _)| k.contract_name == contract_name)
            .count();
        CHALLENGE_CACHE_SIZE
            .with_label_values(&[contract_name])
            .set(contract_challenges as i64);
        CHALLENGE_CACHE_KEYS
            .with_label_values(&[contract_name])
            .set(contract_keys as i64);

        expires_at_unix
    }

    /// Verify and consume a challenge
    ///
    /// Returns Ok(()) if the challenge was valid and has been consumed.
    /// Returns Err with a message if the challenge is invalid or expired.
    ///
    /// IMPORTANT: This method consumes the challenge on success, preventing replay.
    pub fn verify_and_consume(
        &self,
        contract_name: &str,
        chain_id: &ChainId,
        address: &AccountAddress,
        challenge: &str,
    ) -> Result<(), ChallengeError> {
        let key = CacheKey {
            contract_name: contract_name.to_string(),
            chain_id: *chain_id,
            address: *address,
        };

        let mut cache = self.cache.write();

        // Get the challenges for this key
        let challenges = match cache.get_mut(&key) {
            Some(c) => c,
            None => {
                CHALLENGE_CACHE_OPERATIONS
                    .with_label_values(&[contract_name, ChallengeCacheOp::VerifyNotFound.as_str()])
                    .inc();
                return Err(ChallengeError::NotFound);
            },
        };

        // Remove expired challenges first
        challenges.retain(|c| !c.is_expired());

        // Find the matching challenge
        let idx = match challenges.iter().position(|c| c.challenge == challenge) {
            Some(i) => i,
            None => {
                CHALLENGE_CACHE_OPERATIONS
                    .with_label_values(&[contract_name, ChallengeCacheOp::VerifyNotFound.as_str()])
                    .inc();
                return Err(ChallengeError::NotFound);
            },
        };

        // Check if expired (shouldn't happen after retain, but double-check)
        if challenges[idx].is_expired() {
            challenges.remove(idx);
            CHALLENGE_CACHE_OPERATIONS
                .with_label_values(&[contract_name, ChallengeCacheOp::VerifyExpired.as_str()])
                .inc();
            Self::update_size_metrics(&cache, contract_name);
            return Err(ChallengeError::Expired);
        }

        // Remove the challenge (consume it) to prevent replay
        challenges.remove(idx);

        // Clean up empty entries
        if challenges.is_empty() {
            cache.remove(&key);
        }

        // Record successful verification
        CHALLENGE_CACHE_OPERATIONS
            .with_label_values(&[contract_name, ChallengeCacheOp::VerifySuccess.as_str()])
            .inc();
        Self::update_size_metrics(&cache, contract_name);

        Ok(())
    }

    /// Helper to update size metrics after cache mutations (per contract)
    fn update_size_metrics(cache: &HashMap<CacheKey, Vec<CachedChallenge>>, contract_name: &str) {
        let contract_challenges: usize = cache
            .iter()
            .filter(|(k, _)| k.contract_name == contract_name)
            .map(|(_, v)| v.len())
            .sum();
        let contract_keys: usize = cache
            .iter()
            .filter(|(k, _)| k.contract_name == contract_name)
            .count();
        CHALLENGE_CACHE_SIZE
            .with_label_values(&[contract_name])
            .set(contract_challenges as i64);
        CHALLENGE_CACHE_KEYS
            .with_label_values(&[contract_name])
            .set(contract_keys as i64);
    }

    /// Clean up all expired challenges from the cache
    #[allow(dead_code)]
    pub fn cleanup_expired(&self) {
        let mut cache = self.cache.write();
        // Remove expired challenges within each entry
        for challenges in cache.values_mut() {
            challenges.retain(|c| !c.is_expired());
        }
        // Remove empty entries
        cache.retain(|_, v| !v.is_empty());
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let mut total_challenges = 0;
        let mut expired_challenges = 0;

        for challenges in cache.values() {
            for c in challenges {
                total_challenges += 1;
                if c.is_expired() {
                    expired_challenges += 1;
                }
            }
        }

        CacheStats {
            total_keys: cache.len(),
            total_challenges,
            active_challenges: total_challenges - expired_challenges,
            expired_challenges,
        }
    }
}

impl Default for ChallengeCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for challenge verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeError {
    /// Challenge was not found (never issued or already consumed)
    NotFound,
    /// Challenge has expired
    Expired,
}

impl std::fmt::Display for ChallengeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChallengeError::NotFound => {
                write!(f, "challenge not found (not issued or already used)")
            },
            ChallengeError::Expired => write!(f, "challenge has expired"),
        }
    }
}

impl std::error::Error for ChallengeError {}

/// Cache statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CacheStats {
    pub total_keys: usize,
    pub total_challenges: usize,
    pub active_challenges: usize,
    pub expired_challenges: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_store_and_verify_challenge() {
        let cache = ChallengeCache::new();
        let contract = "test_contract";
        let chain_id = ChainId::new(1);
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let challenge = "test-challenge-123";

        // Store challenge
        cache.store_challenge(contract, &chain_id, &addr, challenge.to_string());

        // Verify and consume should succeed
        assert!(cache
            .verify_and_consume(contract, &chain_id, &addr, challenge)
            .is_ok());

        // Second verify should fail (challenge consumed)
        assert_eq!(
            cache.verify_and_consume(contract, &chain_id, &addr, challenge),
            Err(ChallengeError::NotFound)
        );
    }

    #[test]
    fn test_challenge_not_found() {
        let cache = ChallengeCache::new();
        let contract = "test_contract";
        let chain_id = ChainId::new(1);
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();

        // Verify non-existent challenge
        assert_eq!(
            cache.verify_and_consume(contract, &chain_id, &addr, "fake-challenge"),
            Err(ChallengeError::NotFound)
        );
    }

    #[test]
    fn test_wrong_challenge() {
        let cache = ChallengeCache::new();
        let contract = "test_contract";
        let chain_id = ChainId::new(1);
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();

        // Store one challenge
        cache.store_challenge(contract, &chain_id, &addr, "real-challenge".to_string());

        // Verify different challenge should fail
        assert_eq!(
            cache.verify_and_consume(contract, &chain_id, &addr, "fake-challenge"),
            Err(ChallengeError::NotFound)
        );

        // Original challenge should still be valid
        assert!(cache
            .verify_and_consume(contract, &chain_id, &addr, "real-challenge")
            .is_ok());
    }

    #[test]
    fn test_challenge_expiration() {
        let cache = ChallengeCache::with_ttl(Duration::from_millis(50));
        let contract = "test_contract";
        let chain_id = ChainId::new(1);
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let challenge = "expiring-challenge";

        // Store challenge
        cache.store_challenge(contract, &chain_id, &addr, challenge.to_string());

        // Wait for expiration
        sleep(Duration::from_millis(100));

        // Should fail due to expiration
        assert_eq!(
            cache.verify_and_consume(contract, &chain_id, &addr, challenge),
            Err(ChallengeError::NotFound) // NotFound because expired challenges are removed
        );
    }

    #[test]
    fn test_different_contracts_isolated() {
        let cache = ChallengeCache::new();
        let chain_id = ChainId::new(1);
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let challenge = "shared-challenge";

        // Store for contract1
        cache.store_challenge("contract1", &chain_id, &addr, challenge.to_string());

        // Verify for contract2 should fail
        assert_eq!(
            cache.verify_and_consume("contract2", &chain_id, &addr, challenge),
            Err(ChallengeError::NotFound)
        );

        // Verify for contract1 should succeed
        assert!(cache
            .verify_and_consume("contract1", &chain_id, &addr, challenge)
            .is_ok());
    }

    #[test]
    fn test_different_addresses_isolated() {
        let cache = ChallengeCache::new();
        let contract = "test_contract";
        let chain_id = ChainId::new(1);
        let addr1 = AccountAddress::from_hex_literal("0x1").unwrap();
        let addr2 = AccountAddress::from_hex_literal("0x2").unwrap();
        let challenge = "shared-challenge";

        // Store for addr1
        cache.store_challenge(contract, &chain_id, &addr1, challenge.to_string());

        // Verify for addr2 should fail
        assert_eq!(
            cache.verify_and_consume(contract, &chain_id, &addr2, challenge),
            Err(ChallengeError::NotFound)
        );

        // Verify for addr1 should succeed
        assert!(cache
            .verify_and_consume(contract, &chain_id, &addr1, challenge)
            .is_ok());
    }

    #[test]
    fn test_max_challenges_per_address() {
        let cache = ChallengeCache::new();
        let contract = "test_contract";
        let chain_id = ChainId::new(1);
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();

        // Store more than MAX_CHALLENGES_PER_ADDRESS challenges
        for i in 0..=MAX_CHALLENGES_PER_ADDRESS {
            cache.store_challenge(contract, &chain_id, &addr, format!("challenge-{}", i));
        }

        // First challenge should have been evicted
        assert_eq!(
            cache.verify_and_consume(contract, &chain_id, &addr, "challenge-0"),
            Err(ChallengeError::NotFound)
        );

        // Last challenge should still be valid
        let last_challenge = format!("challenge-{}", MAX_CHALLENGES_PER_ADDRESS);
        assert!(cache
            .verify_and_consume(contract, &chain_id, &addr, &last_challenge)
            .is_ok());
    }
}

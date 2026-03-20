// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Rate limiter for unknown/untrusted telemetry.
//!
//! Implements a token bucket algorithm for rate limiting requests from unknown/untrusted nodes.
//! The rate limiter is designed to be concurrent-safe and efficient.

use crate::UnknownTelemetryRateLimitConfig;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Global rate limiter for unknown/untrusted telemetry.
///
/// Uses a token bucket algorithm:
/// - Tokens are added at a rate of `requests_per_second`
/// - Bucket capacity is `burst_capacity`
/// - Each request consumes one token
/// - Requests are rejected when no tokens are available
#[derive(Debug)]
pub struct GlobalRateLimiter {
    /// Configuration for the rate limiter
    config: UnknownTelemetryRateLimitConfig,
    /// Available tokens (stored as tokens * 1000 for sub-token precision)
    tokens_millis: AtomicU64,
    /// Last time tokens were refilled
    last_refill: RwLock<Instant>,
}

impl GlobalRateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: UnknownTelemetryRateLimitConfig) -> Self {
        Self {
            tokens_millis: AtomicU64::new(config.burst_capacity as u64 * 1000),
            last_refill: RwLock::new(Instant::now()),
            config,
        }
    }

    /// Check if a request should be allowed.
    /// Returns true if allowed, false if rate limited.
    pub async fn check_rate_limit(&self) -> bool {
        // If rate limiting is disabled, always allow
        if !self.config.enabled || self.config.requests_per_second == 0 {
            return true;
        }

        // Refill tokens based on elapsed time
        self.refill_tokens().await;

        // Try to consume one token (1000 millis)
        let current = self.tokens_millis.load(Ordering::Relaxed);
        if current >= 1000 {
            // Use compare-and-swap for thread safety
            match self.tokens_millis.compare_exchange(
                current,
                current - 1000,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => true,
                Err(_) => {
                    // Race condition - retry
                    self.check_rate_limit_retry().await
                },
            }
        } else {
            false
        }
    }

    /// Retry rate limit check after a race condition.
    async fn check_rate_limit_retry(&self) -> bool {
        for _ in 0..3 {
            self.refill_tokens().await;
            let current = self.tokens_millis.load(Ordering::Relaxed);
            if current >= 1000 {
                match self.tokens_millis.compare_exchange(
                    current,
                    current - 1000,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return true,
                    Err(_) => continue,
                }
            } else {
                return false;
            }
        }
        false
    }

    /// Refill tokens based on elapsed time.
    async fn refill_tokens(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.write().await;
        let elapsed = now.duration_since(*last_refill);

        if elapsed >= Duration::from_millis(1) {
            // Calculate tokens to add (in millis for precision)
            let tokens_to_add =
                (elapsed.as_millis() as u64 * self.config.requests_per_second as u64) / 1000;

            if tokens_to_add > 0 {
                let max_tokens = self.config.burst_capacity as u64 * 1000;
                let current = self.tokens_millis.load(Ordering::Relaxed);
                let new_tokens = (current + tokens_to_add * 1000).min(max_tokens);
                self.tokens_millis.store(new_tokens, Ordering::Relaxed);
                *last_refill = now;
            }
        }
    }
}

/// Per-contract rate limiter manager.
///
/// Manages rate limiters for each custom contract that has untrusted rate limiting configured.
#[derive(Debug, Default)]
pub struct ContractRateLimiters {
    /// Map of contract name to its rate limiter
    limiters: RwLock<HashMap<String, GlobalRateLimiter>>,
}

impl ContractRateLimiters {
    /// Create a new contract rate limiter manager.
    pub fn new() -> Self {
        Self {
            limiters: RwLock::new(HashMap::new()),
        }
    }

    /// Add a rate limiter for a specific contract.
    pub async fn add_limiter(
        &self,
        contract_name: String,
        config: UnknownTelemetryRateLimitConfig,
    ) {
        let mut limiters = self.limiters.write().await;
        limiters.insert(contract_name, GlobalRateLimiter::new(config));
    }

    /// Check rate limit for a specific contract.
    /// Returns true if allowed, false if rate limited.
    /// Returns true if no specific limiter exists for the contract.
    pub async fn check_rate_limit(&self, contract_name: &str) -> bool {
        let limiters = self.limiters.read().await;
        if let Some(limiter) = limiters.get(contract_name) {
            limiter.check_rate_limit().await
        } else {
            // No specific rate limiter for this contract
            true
        }
    }

    /// Check if a contract has a rate limiter configured.
    pub async fn has_limiter(&self, contract_name: &str) -> bool {
        let limiters = self.limiters.read().await;
        limiters.contains_key(contract_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let config = UnknownTelemetryRateLimitConfig {
            requests_per_second: 10,
            burst_capacity: 5,
            enabled: false,
        };
        let limiter = GlobalRateLimiter::new(config);

        // Should always allow when disabled
        for _ in 0..100 {
            assert!(limiter.check_rate_limit().await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_zero_rps() {
        let config = UnknownTelemetryRateLimitConfig {
            requests_per_second: 0,
            burst_capacity: 5,
            enabled: true,
        };
        let limiter = GlobalRateLimiter::new(config);

        // Should always allow when rps is 0
        for _ in 0..100 {
            assert!(limiter.check_rate_limit().await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_burst() {
        let config = UnknownTelemetryRateLimitConfig {
            requests_per_second: 10,
            burst_capacity: 5,
            enabled: true,
        };
        let limiter = GlobalRateLimiter::new(config);

        // Should allow up to burst capacity
        for _ in 0..5 {
            assert!(limiter.check_rate_limit().await);
        }

        // Should reject after burst is exhausted
        assert!(!limiter.check_rate_limit().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let config = UnknownTelemetryRateLimitConfig {
            requests_per_second: 1000, // High rate for fast refill
            burst_capacity: 1,
            enabled: true,
        };
        let limiter = GlobalRateLimiter::new(config);

        // Exhaust the burst
        assert!(limiter.check_rate_limit().await);
        assert!(!limiter.check_rate_limit().await);

        // Wait for refill (1ms should add ~1 token at 1000 rps)
        tokio::time::sleep(Duration::from_millis(2)).await;

        // Should be allowed after refill
        assert!(limiter.check_rate_limit().await);
    }

    #[tokio::test]
    async fn test_contract_rate_limiters() {
        let manager = ContractRateLimiters::new();

        // No limiter configured - should allow
        assert!(manager.check_rate_limit("unknown_contract").await);

        // Add a limiter
        manager
            .add_limiter(
                "test_contract".to_string(),
                UnknownTelemetryRateLimitConfig {
                    requests_per_second: 10,
                    burst_capacity: 2,
                    enabled: true,
                },
            )
            .await;

        // Should allow up to burst
        assert!(manager.check_rate_limit("test_contract").await);
        assert!(manager.check_rate_limit("test_contract").await);
        assert!(!manager.check_rate_limit("test_contract").await);

        // Other contracts still allowed
        assert!(manager.check_rate_limit("other_contract").await);
    }
}

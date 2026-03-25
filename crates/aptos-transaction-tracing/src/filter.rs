// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::HashValue;
use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Filter that determines which transactions to trace based on sender address
/// and optional probabilistic sampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFilter {
    pub enabled: bool,
    pub sender_allowlist: HashSet<AccountAddress>,
    /// Fraction of allowlisted transactions to trace (0.0–1.0).
    /// 1.0 = trace all matching (default), 0.01 = trace ~1%.
    /// Uses the txn hash for deterministic, stateless sampling.
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

fn default_sample_rate() -> f64 {
    0.01
}

impl TransactionFilter {
    pub fn new(enabled: bool, sender_allowlist: HashSet<AccountAddress>, sample_rate: f64) -> Self {
        Self {
            enabled,
            sender_allowlist,
            sample_rate,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            sender_allowlist: HashSet::new(),
            sample_rate: 0.0,
        }
    }

    /// Returns true if tracing is active (enabled with a non-empty allowlist).
    pub fn is_active(&self) -> bool {
        self.enabled && !self.sender_allowlist.is_empty()
    }

    /// Returns true if the transaction should be traced.
    /// Checks: enabled → sender in allowlist → passes sampling.
    pub fn should_trace(&self, sender: &AccountAddress, hash: &HashValue) -> bool {
        self.enabled && self.sender_allowlist.contains(sender) && self.sample_accepts(hash)
    }

    /// Deterministic, stateless sampling using the txn hash.
    /// The hash is already uniformly distributed, so we just compare
    /// the first 8 bytes against a threshold derived from sample_rate.
    fn sample_accepts(&self, hash: &HashValue) -> bool {
        if self.sample_rate >= 1.0 {
            return true;
        }
        if self.sample_rate <= 0.0 {
            return false;
        }
        let h = u64::from_le_bytes(hash.as_ref()[0..8].try_into().unwrap());
        h <= (self.sample_rate * u64::MAX as f64) as u64
    }
}

impl Default for TransactionFilter {
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_accepts_rate_1() {
        let filter = TransactionFilter::new(true, HashSet::new(), 1.0);
        // rate=1.0 should accept everything
        for i in 0..100u64 {
            let hash = HashValue::from_slice(&{
                let mut buf = [0u8; 32];
                buf[0..8].copy_from_slice(&i.to_le_bytes());
                buf
            })
            .unwrap();
            assert!(filter.sample_accepts(&hash));
        }
    }

    #[test]
    fn test_sample_accepts_rate_0() {
        let filter = TransactionFilter::new(true, HashSet::new(), 0.0);
        // rate=0.0 should reject everything
        for i in 0..100u64 {
            let hash = HashValue::from_slice(&{
                let mut buf = [0u8; 32];
                buf[0..8].copy_from_slice(&i.to_le_bytes());
                buf
            })
            .unwrap();
            assert!(!filter.sample_accepts(&hash));
        }
    }

    #[test]
    fn test_sample_accepts_rate_half() {
        let filter = TransactionFilter::new(true, HashSet::new(), 0.5);
        // With uniform hash distribution, ~50% should be accepted.
        // Use enough samples to be statistically stable.
        let mut accepted = 0;
        for i in 0..10000u64 {
            let hash = HashValue::sha3_256_of(&i.to_le_bytes());
            if filter.sample_accepts(&hash) {
                accepted += 1;
            }
        }
        // Allow 45%-55% range
        assert!(
            (4500..=5500).contains(&accepted),
            "Expected ~50% acceptance, got {}/10000",
            accepted
        );
    }
}

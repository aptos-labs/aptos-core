// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::HashValue;
use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Filter that determines which transactions to trace based on sender address
/// and two-level probabilistic sampling.
///
/// Level 1 (batch): `batch_sample_rate` controls what fraction of QS pull rounds
/// do any tracing work. 90% of rounds skip entirely (~5ns cost).
///
/// Level 2 (txn): `txn_sample_rate` controls what fraction of allowlisted txns
/// within a sampled round are actually traced.
///
/// Effective rate = batch_sample_rate × txn_sample_rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFilter {
    pub enabled: bool,
    pub sender_allowlist: HashSet<AccountAddress>,
    /// Fraction of QS pull rounds that perform tracing (0.0–1.0).
    pub batch_sample_rate: f64,
    /// Fraction of allowlisted txns to trace within a sampled round (0.0–1.0).
    pub txn_sample_rate: f64,
}

impl TransactionFilter {
    pub fn new(
        enabled: bool,
        sender_allowlist: HashSet<AccountAddress>,
        batch_sample_rate: f64,
        txn_sample_rate: f64,
    ) -> Self {
        Self {
            enabled,
            sender_allowlist,
            batch_sample_rate,
            txn_sample_rate,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            sender_allowlist: HashSet::new(),
            batch_sample_rate: 0.0,
            txn_sample_rate: 0.0,
        }
    }

    /// Returns true if tracing is active (enabled with a non-empty allowlist).
    pub fn is_active(&self) -> bool {
        self.enabled && !self.sender_allowlist.is_empty()
    }

    /// Returns true if the transaction should be traced at mempool insertion.
    /// Checks: enabled → sender in allowlist → txn passes txn-level sampling.
    pub fn should_trace(&self, sender: &AccountAddress, hash: &HashValue) -> bool {
        self.enabled
            && self.sender_allowlist.contains(sender)
            && txn_sample_accepts(self.txn_sample_rate, hash)
    }

    /// Returns true if this QS pull round should do tracing work.
    /// Selects N rounds out of every 100 (where N = rate * 100), evenly spaced.
    /// E.g., rate=0.1 → rounds 0,10,20,...,90; rate=0.6 → 60 evenly spread rounds.
    /// Accurate to 0.01 granularity.
    pub fn should_sample_batch(&self, pull_round: u64) -> bool {
        if !self.enabled || self.sender_allowlist.is_empty() {
            return false;
        }
        if self.batch_sample_rate >= 1.0 {
            return true;
        }
        if self.batch_sample_rate <= 0.0 {
            return false;
        }
        let n = (self.batch_sample_rate * 100.0).round() as u64;
        if n >= 100 {
            return true;
        }
        // Bresenham-style even distribution: sample slot i if it crosses
        // a new integer boundary in floor(i*n/100) vs floor((i-1)*n/100).
        // Slot 0 is always sampled when n > 0.
        let slot = pull_round % 100;
        if slot == 0 {
            return true;
        }
        (slot * n / 100) != ((slot - 1) * n / 100)
    }
}

/// Deterministic, stateless sampling for txn-level filtering.
/// Uses the txn hash (already uniformly distributed) compared against a threshold.
/// O(1), ~2ns, no RNG or atomic state.
fn txn_sample_accepts(rate: f64, hash: &HashValue) -> bool {
    if rate >= 1.0 {
        return true;
    }
    if rate <= 0.0 {
        return false;
    }
    let h = u64::from_le_bytes(hash.as_ref()[0..8].try_into().unwrap());
    // Rust 1.45+ saturates f64→u64 cast, so this is safe even for rate ≈ 1.0.
    h <= (rate * u64::MAX as f64) as u64
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
    fn test_txn_sample_rate_1() {
        for i in 0..100u64 {
            let hash = HashValue::sha3_256_of(&i.to_le_bytes());
            assert!(txn_sample_accepts(1.0, &hash));
        }
    }

    #[test]
    fn test_txn_sample_rate_0() {
        for i in 0..100u64 {
            let hash = HashValue::sha3_256_of(&i.to_le_bytes());
            assert!(!txn_sample_accepts(0.0, &hash));
        }
    }

    #[test]
    fn test_txn_sample_rate_half() {
        let mut accepted = 0;
        for i in 0..10000u64 {
            let hash = HashValue::sha3_256_of(&i.to_le_bytes());
            if txn_sample_accepts(0.5, &hash) {
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

    #[test]
    fn test_should_sample_batch_every_10th() {
        let filter = TransactionFilter::new(
            true,
            vec![AccountAddress::ONE].into_iter().collect(),
            0.1,
            1.0,
        );
        let mut sampled = 0;
        for round in 0..10000u64 {
            if filter.should_sample_batch(round) {
                sampled += 1;
            }
        }
        // rate=0.1 → every 10th round → exactly 1000
        assert_eq!(sampled, 1000, "Expected exactly 1000/10000 sampled");
    }
}

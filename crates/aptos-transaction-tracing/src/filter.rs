// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

<<<<<<< HEAD
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
=======
/// Filter that determines which transactions to trace based on sender address.
>>>>>>> 81ead34797 ([tracing] Add aptos-transaction-tracing crate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFilter {
    pub enabled: bool,
    pub sender_allowlist: HashSet<AccountAddress>,
<<<<<<< HEAD
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
=======
}

impl TransactionFilter {
    pub fn new(enabled: bool, sender_allowlist: HashSet<AccountAddress>) -> Self {
        Self {
            enabled,
            sender_allowlist,
>>>>>>> 81ead34797 ([tracing] Add aptos-transaction-tracing crate)
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            sender_allowlist: HashSet::new(),
<<<<<<< HEAD
            batch_sample_rate: 0.0,
            txn_sample_rate: 0.0,
=======
>>>>>>> 81ead34797 ([tracing] Add aptos-transaction-tracing crate)
        }
    }

    /// Returns true if tracing is active (enabled with a non-empty allowlist).
    pub fn is_active(&self) -> bool {
        self.enabled && !self.sender_allowlist.is_empty()
    }

<<<<<<< HEAD
    /// Returns true if the transaction should be traced at mempool insertion.
    /// Checks: enabled → sender in allowlist → txn passes txn-level sampling.
    pub fn should_trace(&self, sender: &AccountAddress, hash: &HashValue) -> bool {
        self.enabled
            && self.sender_allowlist.contains(sender)
            && sample_accepts(self.txn_sample_rate, hash.as_ref())
    }

    /// Returns true if this QS pull round should do tracing work.
    /// Uses pull_round as deterministic coin — no RNG, no locks.
    pub fn should_sample_batch(&self, pull_round: u64) -> bool {
        self.enabled
            && !self.sender_allowlist.is_empty()
            && sample_accepts(self.batch_sample_rate, &pull_round.to_le_bytes())
=======
    /// Returns true if the sender should be traced.
    /// Requires both enabled=true and the sender to be in a non-empty allowlist.
    pub fn should_trace(&self, sender: &AccountAddress) -> bool {
        self.enabled && self.sender_allowlist.contains(sender)
>>>>>>> 81ead34797 ([tracing] Add aptos-transaction-tracing crate)
    }
}

/// Deterministic, stateless sampling. Uses the first 8 bytes of `seed` as a
/// uniform u64, compares against a threshold derived from `rate`.
/// O(1), ~2ns, no RNG or atomic state.
fn sample_accepts(rate: f64, seed: &[u8]) -> bool {
    if rate >= 1.0 {
        return true;
    }
    if rate <= 0.0 {
        return false;
    }
    let mut buf = [0u8; 8];
    let len = seed.len().min(8);
    buf[..len].copy_from_slice(&seed[..len]);
    let h = u64::from_le_bytes(buf);
    // Mix bits for short seeds (e.g., pull_round) to avoid patterns
    let h = h.wrapping_mul(0x517CC1B727220A95);
    h <= (rate * u64::MAX as f64) as u64
}

impl Default for TransactionFilter {
    fn default() -> Self {
        Self::disabled()
    }
}
<<<<<<< HEAD

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_accepts_rate_1() {
        for i in 0..100u64 {
            assert!(sample_accepts(1.0, &i.to_le_bytes()));
        }
    }

    #[test]
    fn test_sample_accepts_rate_0() {
        for i in 0..100u64 {
            assert!(!sample_accepts(0.0, &i.to_le_bytes()));
        }
    }

    #[test]
    fn test_sample_accepts_rate_half() {
        let mut accepted = 0;
        for i in 0..10000u64 {
            if sample_accepts(0.5, &i.to_le_bytes()) {
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
    fn test_should_sample_batch_rate_tenth() {
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
        // Allow 7%-13% range
        assert!(
            (700..=1300).contains(&sampled),
            "Expected ~10% batch sampling, got {}/10000",
            sampled
        );
    }
}
=======
>>>>>>> 81ead34797 ([tracing] Add aptos-transaction-tracing crate)

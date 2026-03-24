// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for transaction lifecycle tracing.
///
/// When `enabled` is true and `sender_allowlist` is non-empty, the node
/// traces transactions from the listed senders through the pipeline
/// (mempool → QS → consensus → execution → commit) and logs a
/// `TxnTrace` line for each.
///
/// Two-level sampling controls overhead:
/// - `batch_sample_rate`: fraction of QS pull rounds that do any tracing work
/// - `txn_sample_rate`: fraction of allowlisted txns traced within a sampled round
/// Effective trace rate = batch_sample_rate × txn_sample_rate.
/// Default: 0.1 × 0.1 = 1% of matching txns, with 90% of pull rounds skipped entirely.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransactionTracingConfig {
    /// Master switch for tracing. Must be true for any tracing to occur.
    pub enabled: bool,
    /// Only transactions from these senders are traced. If empty, nothing
    /// is traced even when `enabled` is true.
    pub sender_allowlist: HashSet<AccountAddress>,
    /// Fraction of QS pull rounds that perform tracing work (0.0–1.0).
    /// Default: 0.1 (10% of rounds). 90% of rounds skip with ~5ns cost.
    pub batch_sample_rate: f64,
    /// Fraction of allowlisted transactions to trace within a sampled round (0.0–1.0).
    /// Default: 0.1 (10% of matching txns). Combined with batch_sample_rate
    /// gives effective 1% trace rate.
    pub txn_sample_rate: f64,
}

impl Default for TransactionTracingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sender_allowlist: HashSet::new(),
            batch_sample_rate: 0.1,
            txn_sample_rate: 0.1,
        }
    }
}

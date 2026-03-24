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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransactionTracingConfig {
    /// Master switch for tracing. Must be true for any tracing to occur.
    pub enabled: bool,
    /// Only transactions from these senders are traced. If empty, nothing
    /// is traced even when `enabled` is true.
    pub sender_allowlist: HashSet<AccountAddress>,
}

impl Default for TransactionTracingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sender_allowlist: HashSet::new(),
        }
    }
}

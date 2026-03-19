// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Filter that determines which transactions to trace based on sender address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFilter {
    pub enabled: bool,
    pub sender_allowlist: HashSet<AccountAddress>,
}

impl TransactionFilter {
    pub fn new(enabled: bool, sender_allowlist: HashSet<AccountAddress>) -> Self {
        Self {
            enabled,
            sender_allowlist,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            sender_allowlist: HashSet::new(),
        }
    }

    /// Returns true if the sender should be traced.
    /// If enabled and the allowlist is empty, all senders are traced.
    pub fn should_trace(&self, sender: &AccountAddress) -> bool {
        self.enabled
            && (self.sender_allowlist.is_empty()
                || self.sender_allowlist.contains(sender))
    }
}

impl Default for TransactionFilter {
    fn default() -> Self {
        Self::disabled()
    }
}

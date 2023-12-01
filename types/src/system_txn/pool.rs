// Copyright © Aptos Foundation

use crate::system_txn::SystemTransaction;
use aptos_crypto::HashValue;
use std::collections::HashSet;

pub enum SystemTransactionFilter {
    PendingTxnHashSet(HashSet<HashValue>),
    RejectAll,
}

pub trait SystemTransactionPoolClient: Send + Sync {
    fn pull(
        &self,
        max_items: u64,
        max_bytes: u64,
        exclude: SystemTransactionFilter,
    ) -> Vec<SystemTransaction>;
}

pub struct SystemTransactionPool {}

impl SystemTransactionPool {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SystemTransactionPool {
    fn default() -> Self {
        Self::new()
    }
}
impl SystemTransactionPoolClient for SystemTransactionPool {
    fn pull(
        &self,
        _max_items: u64,
        _max_bytes: u64,
        _exclude: SystemTransactionFilter,
    ) -> Vec<SystemTransaction> {
        vec![]
    }
}

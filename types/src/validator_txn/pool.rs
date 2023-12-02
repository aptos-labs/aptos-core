// Copyright © Aptos Foundation

use crate::validator_txn::ValidatorTransaction;
use aptos_crypto::HashValue;
use std::collections::HashSet;

pub enum ValidatorTransactionFilter {
    PendingTxnHashSet(HashSet<HashValue>),
}

pub trait ValidatorTransactionPoolClient: Send + Sync {
    fn pull(
        &self,
        max_items: usize,
        max_bytes: usize,
        exclude: ValidatorTransactionFilter,
    ) -> Vec<ValidatorTransaction>;
}

pub struct ValidatorTransactionPool {}

impl ValidatorTransactionPool {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ValidatorTransactionPool {
    fn default() -> Self {
        Self::new()
    }
}
impl ValidatorTransactionPoolClient for ValidatorTransactionPool {
    fn pull(
        &self,
        _max_items: usize,
        _max_bytes: usize,
        _exclude: ValidatorTransactionFilter,
    ) -> Vec<ValidatorTransaction> {
        vec![]
    }
}

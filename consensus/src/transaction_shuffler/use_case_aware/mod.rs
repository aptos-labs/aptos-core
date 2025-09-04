// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::TransactionShuffler;
use velor_types::transaction::{
    signature_verified_transaction::SignatureVerifiedTransaction, use_case::UseCaseKey,
    SignedTransaction,
};
use iterator::ShuffledTransactionIterator;
use std::fmt::Debug;

pub(crate) mod iterator;
pub(crate) mod types;
pub(crate) mod utils;

pub(crate) mod delayed_queue;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub sender_spread_factor: usize,
    pub platform_use_case_spread_factor: usize,
    pub user_use_case_spread_factor: usize,
}

impl Config {
    pub(crate) fn sender_spread_factor(&self) -> usize {
        self.sender_spread_factor
    }

    pub(crate) fn use_case_spread_factor(&self, use_case_key: &UseCaseKey) -> usize {
        use UseCaseKey::*;

        match use_case_key {
            Platform => self.platform_use_case_spread_factor,
            ContractAddress(..) | Others => self.user_use_case_spread_factor,
        }
    }
}

pub struct UseCaseAwareShuffler {
    pub config: Config,
}

#[cfg(feature = "fuzzing")]
impl UseCaseAwareShuffler {
    pub fn shuffle_generic<
        Txn: velor_types::transaction::use_case::UseCaseAwareTransaction + Debug,
    >(
        &self,
        txns: Vec<Txn>,
    ) -> Vec<Txn> {
        ShuffledTransactionIterator::new(self.config.clone())
            .extended_with(txns)
            .collect()
    }
}

impl TransactionShuffler for UseCaseAwareShuffler {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        self.signed_transaction_iterator(txns).collect()
    }

    fn signed_transaction_iterator(
        &self,
        txns: Vec<SignedTransaction>,
    ) -> Box<dyn Iterator<Item = SignedTransaction> + 'static> {
        let iterator = ShuffledTransactionIterator::new(self.config.clone()).extended_with(txns);
        Box::new(iterator)
    }

    fn signature_verified_transaction_iterator(
        &self,
        txns: Vec<SignatureVerifiedTransaction>,
    ) -> Box<dyn Iterator<Item = SignatureVerifiedTransaction> + 'static> {
        let iterator = ShuffledTransactionIterator::new(self.config.clone()).extended_with(txns);
        Box::new(iterator)
    }
}

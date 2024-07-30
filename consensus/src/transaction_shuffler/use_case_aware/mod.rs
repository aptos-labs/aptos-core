// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::TransactionShuffler;
use aptos_types::transaction::{use_case::UseCaseKey, SignedTransaction};
use iterator::ShuffledTransactionIterator;

pub(crate) mod iterator;
pub(crate) mod types;
pub(crate) mod utils;

pub(crate) mod delayed_queue;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Default)]
pub(crate) struct Config {
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

impl TransactionShuffler for UseCaseAwareShuffler {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        ShuffledTransactionIterator::new(self.config.clone())
            .extended_with(txns)
            .collect()
    }
}

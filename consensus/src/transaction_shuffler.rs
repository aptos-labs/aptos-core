// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sender_aware_shuffler::SenderAwareShuffler;
use aptos_logger::info;
use aptos_types::{
    on_chain_config::{
        TransactionShufflerType,
        TransactionShufflerType::{Deprecated, NoShuffling},
    },
    transaction::SignedTransaction,
};
use std::sync::Arc;

/// Interface to shuffle transactions
pub trait TransactionShuffler: Send + Sync {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction>;
}

/// No Op Shuffler to maintain backward compatibility
pub struct NoOpShuffler {}

impl TransactionShuffler for NoOpShuffler {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        txns
    }
}

pub fn create_transaction_shuffler(
    shuffler_type: TransactionShufflerType,
) -> Arc<dyn TransactionShuffler> {
    match shuffler_type {
        NoShuffling => Arc::new(NoOpShuffler {}),
        Deprecated(_) => Arc::new(NoOpShuffler {}),
    }
}

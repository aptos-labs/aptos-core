// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_logger::info;
use velor_types::{
    on_chain_config::TransactionShufflerType,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction,
    },
};
use std::sync::Arc;

mod use_case_aware;
// re-export use case aware shuffler for fuzzer.
#[cfg(feature = "fuzzing")]
pub mod transaction_shuffler_fuzzing {
    pub mod use_case_aware {
        pub use crate::transaction_shuffler::use_case_aware::{Config, UseCaseAwareShuffler};
    }
}

/// Interface to shuffle transactions
pub trait TransactionShuffler: Send + Sync {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction>;

    /// Given a configuration and a vector of SignedTransactions, return an iterator that
    /// produces them in a particular shuffled order.
    fn signed_transaction_iterator(
        &self,
        txns: Vec<SignedTransaction>,
    ) -> Box<dyn Iterator<Item = SignedTransaction> + 'static>;

    /// Given a configuration and a vector of SignatureVerifiedTransaction, return an iterator of
    /// SignatureVerifiedTransaction.
    fn signature_verified_transaction_iterator(
        &self,
        txns: Vec<SignatureVerifiedTransaction>,
    ) -> Box<dyn Iterator<Item = SignatureVerifiedTransaction> + 'static>;
}

/// No Op Shuffler to maintain backward compatibility
pub struct NoOpShuffler {}

impl TransactionShuffler for NoOpShuffler {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        txns
    }

    fn signed_transaction_iterator(
        &self,
        txns: Vec<SignedTransaction>,
    ) -> Box<dyn Iterator<Item = SignedTransaction>> {
        Box::new(txns.into_iter())
    }

    fn signature_verified_transaction_iterator(
        &self,
        txns: Vec<SignatureVerifiedTransaction>,
    ) -> Box<dyn Iterator<Item = SignatureVerifiedTransaction>> {
        Box::new(txns.into_iter())
    }
}

pub fn create_transaction_shuffler(
    shuffler_type: TransactionShufflerType,
) -> Arc<dyn TransactionShuffler> {
    use TransactionShufflerType::*;

    match shuffler_type {
        NoShuffling => {
            info!("Using no-op transaction shuffling");
            Arc::new(NoOpShuffler {})
        },
        DeprecatedSenderAwareV1(_) => {
            info!("Using no-op sender aware shuffling v1");
            Arc::new(NoOpShuffler {})
        },
        SenderAwareV2(_) => {
            unreachable!("SenderAware shuffler is no longer supported.")
        },
        DeprecatedFairness => {
            unreachable!("DeprecatedFairness shuffler is no longer supported.")
        },
        UseCaseAware {
            sender_spread_factor,
            platform_use_case_spread_factor,
            user_use_case_spread_factor,
        } => {
            let config = use_case_aware::Config {
                sender_spread_factor,
                platform_use_case_spread_factor,
                user_use_case_spread_factor,
            };
            info!(
                config = ?config,
                "Using use case aware transaction shuffling."
            );
            Arc::new(use_case_aware::UseCaseAwareShuffler { config })
        },
    }
}

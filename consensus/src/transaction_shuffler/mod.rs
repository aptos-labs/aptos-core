// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::info;
use aptos_types::{on_chain_config::TransactionShufflerType, transaction::SignedTransaction};
use sender_aware::SenderAwareShuffler;
use std::sync::Arc;

mod deprecated_fairness;
mod sender_aware;
mod use_case_aware;

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
        SenderAwareV2(conflict_window_size) => {
            info!(
                "Using sender aware transaction shuffling with conflict window size {}",
                conflict_window_size
            );
            Arc::new(SenderAwareShuffler::new(conflict_window_size as usize))
        },
        DeprecatedFairness {
            sender_conflict_window_size,
            module_conflict_window_size,
            entry_fun_conflict_window_size,
        } => {
            info!(
                "Using fairness transaction shuffling with conflict window sizes: sender {}, module {}, entry fun {}",
                sender_conflict_window_size,
                module_conflict_window_size,
                entry_fun_conflict_window_size
            );
            Arc::new(deprecated_fairness::FairnessShuffler {
                sender_conflict_window_size: sender_conflict_window_size as usize,
                module_conflict_window_size: module_conflict_window_size as usize,
                entry_fun_conflict_window_size: entry_fun_conflict_window_size as usize,
            })
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

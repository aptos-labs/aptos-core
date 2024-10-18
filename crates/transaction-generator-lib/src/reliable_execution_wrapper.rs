// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::{ReliableTransactionSubmitter, TransactionGenerator, TransactionGeneratorCreator};
use aptos_sdk::types::{transaction::SignedTransaction, LocalAccount};
use async_trait::async_trait;
use aptos_logger::warn;

/// Wrapper that allows inner transaction generator to have unique accounts
/// for all transactions (instead of having 5-20 transactions per account, as default)
/// This is achieved via using accounts from the pool that account creatin can fill,
/// and burning (removing accounts from the pool) them - basically using them only once.
/// (we cannot use more as sequence number is not updated on failure)
pub struct ReliableExecutionWrapperGenerator {
    generator: Box<dyn TransactionGenerator>,
    txn_executor: Arc<dyn ReliableTransactionSubmitter>,
}

impl ReliableExecutionWrapperGenerator {
    pub fn new(
        generator: Box<dyn TransactionGenerator>,
        txn_executor: Arc<dyn ReliableTransactionSubmitter>,
    ) -> Self {
        Self {
            generator,
            txn_executor,
        }
    }
}

#[async_trait]
impl TransactionGenerator for ReliableExecutionWrapperGenerator {
    async fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        loop {
            let txns = self.generator.generate_transactions(account, num_to_create).await;
            if txns.is_empty() {
                return Vec::new();
            }
            if let Err(e) = self.txn_executor
                .execute_transactions(&txns)
                .await {
                    warn!("Error executing transactions reliably: {:?}", e);
                    continue;
                }
        }
    }
}

pub struct ReliableExecutionWrapperCreator {
    creator: Box<dyn TransactionGeneratorCreator>,
    txn_executor: Arc<dyn ReliableTransactionSubmitter>,
}

impl ReliableExecutionWrapperCreator {
    pub fn new(
        creator: Box<dyn TransactionGeneratorCreator>,
        txn_executor: Arc<dyn ReliableTransactionSubmitter>,
    ) -> Self {
        Self {
            creator,
            txn_executor,
        }
    }
}

impl TransactionGeneratorCreator for ReliableExecutionWrapperCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(ReliableExecutionWrapperGenerator::new(
            self.creator.create_transaction_generator(),
            self.txn_executor.clone(),
        ))
    }
}

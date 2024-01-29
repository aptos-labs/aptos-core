// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_sdk::types::{transaction::SignedTransaction, LocalAccount};

struct ReducedBatchWrapperTransactionGenerator {
    batch_size: usize,
    generator: Box<dyn TransactionGenerator>,
}

impl TransactionGenerator for ReducedBatchWrapperTransactionGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        self.generator
            .generate_transactions(account, num_to_create.min(self.batch_size))
    }
}

pub struct ReducedBatchWrapperTransactionGeneratorCreator {
    batch_size: usize,
    generator_creator: Box<dyn TransactionGeneratorCreator>,
}

impl ReducedBatchWrapperTransactionGeneratorCreator {
    #[allow(unused)]
    pub fn new(batch_size: usize, generator_creator: Box<dyn TransactionGeneratorCreator>) -> Self {
        Self {
            batch_size,
            generator_creator,
        }
    }
}

impl TransactionGeneratorCreator for ReducedBatchWrapperTransactionGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(ReducedBatchWrapperTransactionGenerator {
            batch_size: self.batch_size,
            generator: self.generator_creator.create_transaction_generator(),
        })
    }
}

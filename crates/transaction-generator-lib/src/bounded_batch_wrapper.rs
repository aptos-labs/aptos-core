// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{TransactionGenerator, TransactionGeneratorCreator};
use velor_sdk::types::{transaction::SignedTransaction, LocalAccount};

struct BoundedBatchWrapperTransactionGenerator {
    batch_size: usize,
    generator: Box<dyn TransactionGenerator>,
}

impl TransactionGenerator for BoundedBatchWrapperTransactionGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        self.generator
            .generate_transactions(account, num_to_create.min(self.batch_size))
    }
}

pub struct BoundedBatchWrapperTransactionGeneratorCreator {
    batch_size: usize,
    generator_creator: Box<dyn TransactionGeneratorCreator>,
}

impl BoundedBatchWrapperTransactionGeneratorCreator {
    #[allow(unused)]
    pub fn new(batch_size: usize, generator_creator: Box<dyn TransactionGeneratorCreator>) -> Self {
        Self {
            batch_size,
            generator_creator,
        }
    }
}

impl TransactionGeneratorCreator for BoundedBatchWrapperTransactionGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(BoundedBatchWrapperTransactionGenerator {
            batch_size: self.batch_size,
            generator: self.generator_creator.create_transaction_generator(),
        })
    }
}
